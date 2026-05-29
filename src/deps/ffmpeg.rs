use crate::config;

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

    let output = std::process::Command::new(ffmpeg_path)
        .arg("-version")
        .output();

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

fn get_major_version(version_output: &str) -> Option<u8> {
    let first_line = version_output.lines().next()?;
    let version = first_line.split_whitespace().nth(2)?;
    let major = version.split('.').next()?;
    major.parse().ok()
}
