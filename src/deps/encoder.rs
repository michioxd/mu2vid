use crate::config;
use crate::media::thumbnail;
use crate::ui::new_queue::{ORIGINAL_AUDIO_CODEC, QueueItemDraft};
use crate::ui::utils::is_audio_file;
use crate::youtube::{oauth, token, upload};
use anyhow::{Context, Result};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::prelude::Accessor;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone)]
pub struct RenderRequest {
    pub work_dir: PathBuf,
    pub queues: Vec<QueueItemDraft>,
    pub skip_youtube_upload: bool,
}

#[derive(Debug, Clone)]
pub enum RenderEvent {
    QueueStarted {
        index: usize,
        title: String,
    },
    Progress {
        index: usize,
        queue_percent: u32,
        total_percent: u32,
    },
    QueueFinished {
        index: usize,
        output_path: PathBuf,
        will_upload: bool,
    },
    UploadStarted {
        index: usize,
        title: String,
    },
    UploadProgress {
        index: usize,
        uploaded: u64,
        total: u64,
        queue_percent: u32,
        total_percent: u32,
        bytes_per_second: Option<f64>,
    },
    UploadFinished {
        index: usize,
        video_id: Option<String>,
    },
    Finished,
    Cancelled,
    Error {
        index: Option<usize>,
        message: String,
    },
}

#[derive(Debug, Clone)]
struct TrackInfo {
    path: PathBuf,
    disk: u32,
    track: u32,
    artist: String,
    title: String,
    duration: Duration,
}

enum UploadMessage {
    Progress(u64, u64),
    Finished(Result<Option<String>>),
}

const RENDER_PROGRESS_WEIGHT: u32 = 50;
const PROGRESS_IDLE_TIMEOUT: Duration = Duration::from_millis(500);

pub fn render_project(
    request: RenderRequest,
    cancel: Arc<AtomicBool>,
    mut on_event: impl FnMut(RenderEvent),
) {
    if let Err(err) = render_project_inner(request, cancel, &mut on_event) {
        on_event(RenderEvent::Error {
            index: None,
            message: err.to_string(),
        });
    }
}

fn render_project_inner(
    request: RenderRequest,
    cancel: Arc<AtomicBool>,
    on_event: &mut impl FnMut(RenderEvent),
) -> Result<()> {
    fs::create_dir_all(&request.work_dir).context("Cannot create work directory")?;

    let total_queues = request.queues.len().max(1) as u32;
    for (index, item) in request.queues.iter().enumerate() {
        if item.render_status.is_finished() || item.skip_render {
            continue;
        }

        if cancel.load(AtomicOrdering::Relaxed) {
            on_event(RenderEvent::Cancelled);
            return Ok(());
        }

        on_event(RenderEvent::QueueStarted {
            index,
            title: item.title.clone(),
        });

        let will_upload = youtube_upload_enabled(&request, item);

        match render_queue(
            item,
            &request.work_dir,
            index,
            total_queues,
            will_upload,
            &cancel,
            on_event,
        ) {
            Ok(output_path) => {
                on_event(RenderEvent::QueueFinished {
                    index,
                    output_path: output_path.clone(),
                    will_upload,
                });

                if will_upload
                    && let Err(err) =
                        upload_queue_video(item, &output_path, index, total_queues, on_event)
                {
                    on_event(RenderEvent::Error {
                        index: Some(index),
                        message: err.to_string(),
                    });
                    return Ok(());
                }
            }
            Err(err) => {
                on_event(RenderEvent::Error {
                    index: Some(index),
                    message: err.to_string(),
                });
                return Ok(());
            }
        }
    }

    on_event(RenderEvent::Finished);
    Ok(())
}

fn render_queue(
    item: &QueueItemDraft,
    work_dir: &Path,
    index: usize,
    total_queues: u32,
    will_upload: bool,
    cancel: &AtomicBool,
    on_event: &mut impl FnMut(RenderEvent),
) -> Result<PathBuf> {
    let queue_dir = work_dir.join(safe_folder_name(&item.title));
    fs::create_dir_all(&queue_dir)
        .with_context(|| format!("Cannot create queue folder: {}", queue_dir.display()))?;

    let tracks = scan_tracks(Path::new(&item.album_path))?;
    if tracks.is_empty() {
        anyhow::bail!("No audio files found in {}", item.album_path);
    }

    let total_duration = tracks
        .iter()
        .map(|track| track.duration.as_secs_f64())
        .sum::<f64>()
        .max(1.0);

    write_timestamps(&queue_dir.join("timestamps.txt"), &tracks)?;
    thumbnail::generate_thumbnail(&item.artwork_path, &queue_dir.join("thumbnail.jpg"))?;
    let concat_input = build_concat_input(&tracks);

    let output_path = queue_dir.join("output.mp4");
    let mut ffmpeg_command = Command::new(ffmpeg_path());
    hide_console_window(&mut ffmpeg_command);
    let mut ffmpeg = ffmpeg_command
        .args(ffmpeg_args(item, &output_path))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("Cannot start FFmpeg")?;

    let stdin_result = ffmpeg.stdin.take().map(|mut stdin| {
        stdin
            .write_all(concat_input.as_bytes())
            .context("Cannot write FFmpeg concat input")
    });

    let stderr = ffmpeg.stderr.take().context("Cannot read FFmpeg stderr")?;
    let (progress_tx, progress_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if progress_tx.send(line.clone()).is_err() {
                        break;
                    }
                }
                Err(err) => {
                    let _ = progress_tx.send(format!("FFmpeg progress read error: {err}"));
                    break;
                }
            }
        }
    });

    let mut stderr_tail = VecDeque::new();
    let started_at = Instant::now();
    let mut last_queue_percent = 0;

    loop {
        if cancel.load(AtomicOrdering::Relaxed) {
            let _ = ffmpeg.kill();
            on_event(RenderEvent::Cancelled);
            return Ok(output_path);
        }

        match progress_rx.recv_timeout(PROGRESS_IDLE_TIMEOUT) {
            Ok(line) => {
                if let Some(seconds) = parse_ffmpeg_time_seconds(&line) {
                    let render_percent = percent(seconds, total_duration);
                    let queue_percent = queue_render_percent(render_percent, will_upload);
                    last_queue_percent = emit_progress(
                        index,
                        queue_percent,
                        total_queues,
                        on_event,
                        last_queue_percent,
                    );
                } else {
                    push_ffmpeg_log_line(&mut stderr_tail, &line);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if let Some(status) = ffmpeg.try_wait().context("Cannot check FFmpeg status")? {
                    if !status.success() {
                        anyhow::bail!(format_ffmpeg_error(
                            "FFmpeg failed",
                            Some(status),
                            &stderr_tail,
                        ));
                    }
                    break;
                }

                let elapsed_percent = percent(started_at.elapsed().as_secs_f64(), total_duration);
                let queue_percent = queue_render_percent(elapsed_percent, will_upload);
                last_queue_percent = emit_progress(
                    index,
                    queue_percent,
                    total_queues,
                    on_event,
                    last_queue_percent,
                );
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let status = ffmpeg.wait().context("Cannot wait for FFmpeg")?;
    if let Some(Err(err)) = stdin_result {
        anyhow::bail!(format_ffmpeg_error(
            &format!("{err}"),
            Some(status),
            &stderr_tail,
        ));
    }

    if !status.success() {
        anyhow::bail!(format_ffmpeg_error(
            "FFmpeg failed",
            Some(status),
            &stderr_tail,
        ));
    }

    emit_progress(
        index,
        queue_render_percent(100, will_upload),
        total_queues,
        on_event,
        last_queue_percent,
    );
    Ok(output_path)
}

fn emit_progress(
    index: usize,
    queue_percent: u32,
    total_queues: u32,
    on_event: &mut impl FnMut(RenderEvent),
    last_queue_percent: u32,
) -> u32 {
    let queue_percent = queue_percent.max(last_queue_percent).min(100);
    on_event(RenderEvent::Progress {
        index,
        queue_percent,
        total_percent: queue_total_percent(index, queue_percent, total_queues),
    });
    queue_percent
}

fn upload_queue_video(
    item: &QueueItemDraft,
    output_path: &Path,
    index: usize,
    total_queues: u32,
    on_event: &mut impl FnMut(RenderEvent),
) -> Result<()> {
    let mut config = config::load();
    if config.youtube.client_id.trim().is_empty()
        || config.youtube.client_secret.trim().is_empty()
        || config.youtube.refresh_token.is_none()
    {
        return Ok(());
    }

    on_event(RenderEvent::UploadStarted {
        index,
        title: item.title.clone(),
    });

    let (tx, rx) = std::sync::mpsc::channel();
    let output_path = output_path.to_path_buf();
    let title = item.title.clone();
    let description = rendered_description(item, &output_path)?;
    let privacy_status = youtube_privacy_status(&config.youtube.upload_visibility).to_string();
    let thumbnail_path = output_path
        .parent()
        .context("Missing rendered output folder")?
        .join("thumbnail.jpg");
    let total_size = fs::metadata(&output_path)
        .with_context(|| format!("Cannot read video file metadata: {}", output_path.display()))?
        .len();

    std::thread::spawn(move || {
        let result = (|| -> Result<Option<String>> {
            let runtime =
                tokio::runtime::Runtime::new().context("Cannot start YouTube upload runtime")?;
            let access_token = runtime.block_on(valid_access_token(&mut config))?;
            let progress_tx = tx.clone();
            let upload_response = runtime.block_on(upload::upload_video_resumable(
                &access_token,
                &output_path,
                &title,
                &description,
                &privacy_status,
                move |uploaded, total| {
                    let _ = progress_tx.send(UploadMessage::Progress(uploaded, total));
                },
            ))?;
            if let Some(video_id) = upload_response.id.as_deref() {
                runtime.block_on(upload::upload_thumbnail(
                    &access_token,
                    video_id,
                    &thumbnail_path,
                ))?;
            }
            Ok(upload_response.id)
        })();

        let _ = tx.send(UploadMessage::Finished(result));
    });

    on_event(RenderEvent::UploadProgress {
        index,
        uploaded: 0,
        total: total_size,
        queue_percent: queue_upload_percent(0),
        total_percent: queue_total_percent(index, queue_upload_percent(0), total_queues),
        bytes_per_second: None,
    });

    let mut last_progress: Option<(u64, Instant)> = None;
    while let Ok(message) = rx.recv() {
        match message {
            UploadMessage::Progress(uploaded, total) => {
                let now = Instant::now();
                let bytes_per_second = last_progress.and_then(|(last_uploaded, last_time)| {
                    let elapsed = now.duration_since(last_time).as_secs_f64();
                    if elapsed > 0.0 && uploaded >= last_uploaded {
                        Some((uploaded - last_uploaded) as f64 / elapsed)
                    } else {
                        None
                    }
                });
                last_progress = Some((uploaded, now));

                let upload_percent = if total == 0 {
                    0
                } else {
                    ((uploaded.saturating_mul(100)) / total).min(100) as u32
                };
                let queue_percent = queue_upload_percent(upload_percent);
                on_event(RenderEvent::UploadProgress {
                    index,
                    uploaded,
                    total,
                    queue_percent,
                    total_percent: queue_total_percent(index, queue_percent, total_queues),
                    bytes_per_second,
                });
            }
            UploadMessage::Finished(result) => {
                let video_id = result?;
                on_event(RenderEvent::UploadProgress {
                    index,
                    uploaded: total_size,
                    total: total_size,
                    queue_percent: 100,
                    total_percent: queue_total_percent(index, 100, total_queues),
                    bytes_per_second: None,
                });
                on_event(RenderEvent::UploadFinished { index, video_id });
                break;
            }
        }
    }

    Ok(())
}

fn youtube_upload_enabled(request: &RenderRequest, item: &QueueItemDraft) -> bool {
    if request.skip_youtube_upload || item.skip_render {
        return false;
    }

    let config = config::load();
    !config.youtube.client_id.trim().is_empty()
        && !config.youtube.client_secret.trim().is_empty()
        && config.youtube.refresh_token.is_some()
}

fn youtube_privacy_status(value: &str) -> &str {
    if value.eq_ignore_ascii_case("private") {
        "private"
    } else {
        "public"
    }
}

fn queue_render_percent(render_percent: u32, will_upload: bool) -> u32 {
    if will_upload {
        render_percent.min(100) * RENDER_PROGRESS_WEIGHT / 100
    } else {
        render_percent.min(100)
    }
}

fn queue_upload_percent(upload_percent: u32) -> u32 {
    RENDER_PROGRESS_WEIGHT + (upload_percent.min(100) * (100 - RENDER_PROGRESS_WEIGHT) / 100)
}

fn queue_total_percent(index: usize, queue_percent: u32, total_queues: u32) -> u32 {
    (((index as u32) * 100) + queue_percent.min(100)) / total_queues.max(1)
}

fn rendered_description(item: &QueueItemDraft, output_path: &Path) -> Result<String> {
    let timestamps_path = output_path
        .parent()
        .context("Missing rendered output folder")?
        .join("timestamps.txt");
    let timestamps = fs::read_to_string(&timestamps_path)
        .with_context(|| format!("Cannot read timestamps: {}", timestamps_path.display()))?;

    Ok(item.description.replace("{{timestamp}}", &timestamps))
}

async fn valid_access_token(config: &mut config::AppConfig) -> Result<String> {
    if let Some(access_token) = config.youtube.access_token.clone()
        && !token::is_access_token_expired(config.youtube.expires_at)
    {
        return Ok(access_token);
    }

    let refresh_token = config
        .youtube
        .refresh_token
        .clone()
        .context("Missing YouTube refresh token")?;
    let token = oauth::refresh_access_token(
        &config.youtube.client_id,
        &config.youtube.client_secret,
        &refresh_token,
    )
    .await?;

    config.youtube.access_token = Some(token.access_token.clone());
    config.youtube.refresh_token = token.refresh_token;
    config.youtube.expires_at = token.expires_at;
    config::save(config).context("Failed to save refreshed YouTube token")?;

    Ok(token.access_token)
}

fn scan_tracks(album_path: &Path) -> Result<Vec<TrackInfo>> {
    let mut tracks = fs::read_dir(album_path)
        .with_context(|| format!("Cannot read album folder: {}", album_path.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_audio_file(path))
        .map(read_track_info)
        .collect::<Result<Vec<_>>>()?;

    tracks.sort_by(|left, right| match left.disk.cmp(&right.disk) {
        Ordering::Equal => match left.track.cmp(&right.track) {
            Ordering::Equal => left.path.cmp(&right.path),
            value => value,
        },
        value => value,
    });

    Ok(tracks)
}

fn read_track_info(path: PathBuf) -> Result<TrackInfo> {
    let tagged_file = lofty::read_from_path(&path)
        .with_context(|| format!("Cannot read metadata: {}", path.display()))?;
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    let file_name = path
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown title".to_string());

    Ok(TrackInfo {
        path,
        disk: tag.and_then(|tag| tag.disk()).unwrap_or(1),
        track: tag.and_then(|tag| tag.track()).unwrap_or(0),
        artist: tag
            .and_then(|tag| tag.artist())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "Unknown Artist".to_string()),
        title: tag
            .and_then(|tag| tag.title())
            .map(|value| value.to_string())
            .unwrap_or(file_name),
        duration: tagged_file.properties().duration(),
    })
}

fn write_timestamps(path: &Path, tracks: &[TrackInfo]) -> Result<()> {
    let mut current_seconds = 0.0;
    let mut lines = Vec::new();

    for track in tracks {
        lines.push(format!(
            "{} {}.{:02}. {} - {}",
            format_time(current_seconds),
            track.disk,
            track.track,
            track.artist,
            track.title
        ));
        current_seconds += track.duration.as_secs_f64();
    }

    fs::write(path, lines.join("\n")).with_context(|| format!("Cannot write {}", path.display()))
}

fn build_concat_input(tracks: &[TrackInfo]) -> String {
    let mut input = tracks
        .iter()
        .map(|track| format!("file '{}'", escape_concat_path(&track.path)))
        .collect::<Vec<_>>()
        .join("\n");
    input.push('\n');
    input
}

fn ffmpeg_args(item: &QueueItemDraft, output_path: &Path) -> Vec<String> {
    let size = parse_video_size(&item.video_quality);
    let mut args = vec![
        "-y".to_string(),
        "-hide_banner".to_string(),
        "-nostats".to_string(),
        "-progress".to_string(),
        "pipe:2".to_string(),
        "-loop".to_string(),
        "1".to_string(),
        "-framerate".to_string(),
        "1".to_string(),
        "-i".to_string(),
        item.artwork_path.to_string_lossy().to_string(),
        "-protocol_whitelist".to_string(),
        "file,pipe".to_string(),
        "-f".to_string(),
        "concat".to_string(),
        "-safe".to_string(),
        "0".to_string(),
        "-i".to_string(),
        "pipe:0".to_string(),
        "-vf".to_string(),
        format!(
            "scale={size}:{size}:force_original_aspect_ratio=decrease,pad={size}:{size}:(ow-iw)/2:(oh-ih)/2"
        ),
        "-c:v".to_string(),
        video_encoder(),
        "-preset".to_string(),
        "medium".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
    ];

    if item.audio_codec == ORIGINAL_AUDIO_CODEC {
        args.extend(["-c:a".to_string(), "copy".to_string()]);
    } else {
        args.extend([
            "-c:a".to_string(),
            item.audio_codec.clone(),
            "-b:a".to_string(),
            format!("{}k", item.audio_bitrate_kbps),
        ]);
    }

    args.extend([
        "-shortest".to_string(),
        output_path.to_string_lossy().to_string(),
    ]);
    args
}

fn ffmpeg_path() -> String {
    config::load()
        .ffmpeg_path
        .filter(|path| !path.trim().is_empty())
        .unwrap_or_else(|| "ffmpeg".to_string())
}

fn hide_console_window(command: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
}

fn video_encoder() -> String {
    config::load()
        .encoder
        .video_encoder
        .filter(|encoder| !encoder.trim().is_empty())
        .unwrap_or_else(|| "libx264".to_string())
}

fn parse_video_size(quality: &str) -> u32 {
    quality.trim_end_matches('p').parse::<u32>().unwrap_or(1080)
}

fn percent(current: f64, total: f64) -> u32 {
    ((current / total) * 100.0).clamp(0.0, 100.0).round() as u32
}

fn push_ffmpeg_log_line(lines: &mut VecDeque<String>, line: &str) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }

    if lines.len() == 20 {
        lines.pop_front();
    }
    lines.push_back(line.to_string());
}

fn format_ffmpeg_error(
    message: &str,
    status: Option<ExitStatus>,
    stderr_tail: &VecDeque<String>,
) -> String {
    let mut parts = vec![message.to_string()];

    if let Some(status) = status {
        parts.push(format!("status: {status}"));
    }

    if !stderr_tail.is_empty() {
        parts.push(format!(
            "FFmpeg output:\n{}",
            stderr_tail.iter().cloned().collect::<Vec<_>>().join("\n")
        ));
    }

    parts.join("\n")
}

fn parse_ffmpeg_time_seconds(line: &str) -> Option<f64> {
    let value = line.strip_prefix("out_time_ms=")?;
    let micros = value.trim().parse::<f64>().ok()?;
    Some(micros / 1_000_000.0)
}

fn format_time(total_seconds: f64) -> String {
    let rounded = total_seconds.round() as u64;
    let hours = rounded / 3600;
    let minutes = (rounded % 3600) / 60;
    let seconds = rounded % 60;

    if hours == 0 {
        format!("{minutes:02}:{seconds:02}")
    } else {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    }
}

fn escape_concat_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .replace('\'', "'\\''")
}

fn safe_folder_name(value: &str) -> String {
    let name = value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect::<String>()
        .trim()
        .trim_matches('.')
        .to_string();

    if name.is_empty() {
        "queue".to_string()
    } else {
        name
    }
}
