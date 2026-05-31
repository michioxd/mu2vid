use crate::config;
use std::process::Command;

/// Check if FFmpeg is installed and has a supported version.
///
/// Returns:
/// - `1` if FFmpeg is missing or cannot be executed
/// - `2` if FFmpeg is older than 8.0
/// - `3` if FFmpeg is available and version is at least 8.0
/// - `0` for any other unexpected result
pub fn check_ffmpeg() -> i8 {
    let config = config::load();
    let ffmpeg_path = config.ffmpeg_path.as_deref().unwrap_or("ffmpeg");

    check_ffmpeg_path(ffmpeg_path)
}

pub fn check_ffmpeg_path(ffmpeg_path: &str) -> i8 {
    let ffmpeg_path = normalize_ffmpeg_path(ffmpeg_path);

    let output = Command::new(ffmpeg_path).arg("-version").output();

    match output {
        Ok(output) => {
            if !output.status.success() {
                return 1;
            }

            let version_output = String::from_utf8_lossy(&output.stdout);
            match get_major_version(&version_output) {
                Some(version) if version >= 8 => 3,
                Some(_) => 2,
                None => 0,
            }
        }
        Err(_) => 1,
    }
}

pub fn video_encoders(ffmpeg_path: &str) -> anyhow::Result<Vec<String>> {
    let ffmpeg_path = normalize_ffmpeg_path(ffmpeg_path);
    let output = Command::new(ffmpeg_path)
        .args(["-hide_banner", "-encoders"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("ffmpeg -encoders failed");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut encoders = stdout
        .lines()
        .filter_map(parse_video_encoder_line)
        .collect::<Vec<_>>();
    encoders.sort();
    encoders.dedup();
    Ok(encoders)
}

fn normalize_ffmpeg_path(ffmpeg_path: &str) -> &str {
    let ffmpeg_path = ffmpeg_path.trim();

    if ffmpeg_path.is_empty() {
        "ffmpeg"
    } else {
        ffmpeg_path
    }
}

fn parse_video_encoder_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.len() < 8 || !trimmed.starts_with('V') {
        return None;
    }

    let encoder = trimmed.split_whitespace().nth(1)?;
    is_dedicated_video_encoder(encoder).then(|| encoder.to_string())
}

fn is_dedicated_video_encoder(encoder: &str) -> bool {
    let encoder = encoder.to_ascii_lowercase();

    if matches!(
        encoder.as_str(),
        "apng"
            | "bmp"
            | "gif"
            | "jpeg2000"
            | "jpegls"
            | "ljpeg"
            | "mjpeg"
            | "png"
            | "rawvideo"
            | "sgi"
            | "targa"
            | "tiff"
            | "webp"
            | "wrapped_avframe"
    ) {
        return false;
    }

    matches!(
        encoder.as_str(),
        "a64multi"
            | "a64multi5"
            | "cinepak"
            | "dnxhd"
            | "dvvideo"
            | "ffv1"
            | "ffvhuff"
            | "flv"
            | "h261"
            | "h263"
            | "h263p"
            | "huffyuv"
            | "libaom-av1"
            | "libkvazaar"
            | "librav1e"
            | "libsvtav1"
            | "libtheora"
            | "libvpx"
            | "libvpx-vp9"
            | "libx264"
            | "libx264rgb"
            | "libx265"
            | "mpeg1video"
            | "mpeg2video"
            | "mpeg4"
            | "msmpeg4"
            | "msmpeg4v2"
            | "prores"
            | "prores_aw"
            | "prores_ks"
            | "rv10"
            | "rv20"
            | "snow"
            | "svq1"
            | "vp8"
            | "vp9"
            | "wmv1"
            | "wmv2"
    ) || encoder.contains("264")
        || encoder.contains("265")
        || encoder.contains("av1")
        || encoder.contains("hevc")
        || encoder.ends_with("_amf")
        || encoder.ends_with("_mf")
        || encoder.ends_with("_nvenc")
        || encoder.ends_with("_qsv")
        || encoder.ends_with("_vaapi")
        || encoder.ends_with("_v4l2m2m")
        || encoder.ends_with("_videotoolbox")
}

fn get_major_version(version_output: &str) -> Option<u8> {
    let first_line = version_output.lines().next()?;
    let version = first_line.split_whitespace().nth(2)?;
    let major = version.split('.').next()?;
    major.parse().ok()
}
