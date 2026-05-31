use crate::config;
use crate::media::thumbnail;
use crate::ui::new_queue::{ORIGINAL_AUDIO_CODEC, QueueItemDraft};
use crate::ui::utils::is_audio_file;
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
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RenderRequest {
    pub work_dir: PathBuf,
    pub queues: Vec<QueueItemDraft>,
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
        if item.render_status.is_finished() {
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

        match render_queue(
            item,
            &request.work_dir,
            index,
            total_queues,
            &cancel,
            on_event,
        ) {
            Ok(output_path) => on_event(RenderEvent::QueueFinished { index, output_path }),
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
    let mut ffmpeg = Command::new(ffmpeg_path())
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
    let mut reader = std::io::BufReader::new(stderr);
    let mut line = String::new();
    let mut stderr_tail = VecDeque::new();

    loop {
        if cancel.load(AtomicOrdering::Relaxed) {
            let _ = ffmpeg.kill();
            on_event(RenderEvent::Cancelled);
            return Ok(output_path);
        }

        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }

        if let Some(seconds) = parse_ffmpeg_time_seconds(&line) {
            let queue_percent = percent(seconds, total_duration);
            let total_percent = (((index as u32) * 100) + queue_percent) / total_queues;
            on_event(RenderEvent::Progress {
                index,
                queue_percent,
                total_percent,
            });
        } else {
            push_ffmpeg_log_line(&mut stderr_tail, &line);
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

    on_event(RenderEvent::Progress {
        index,
        queue_percent: 100,
        total_percent: (((index as u32) + 1) * 100) / total_queues,
    });
    Ok(output_path)
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
