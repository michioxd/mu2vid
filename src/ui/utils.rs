use std::path::Path;
use std::time::Duration;

pub fn format_file_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;

    if bytes as f64 >= MB {
        format!("{:.2} MB", bytes as f64 / MB)
    } else if bytes as f64 >= KB {
        format!("{:.1} KB", bytes as f64 / KB)
    } else {
        format!("{bytes} B")
    }
}

pub fn open_file_location(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .arg("/select,")
            .arg(path)
            .spawn();
    }

    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn();
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let folder = path.parent().unwrap_or(path);
        let _ = std::process::Command::new("xdg-open").arg(folder).spawn();
    }
}

pub fn double_click_interval() -> Duration {
    #[cfg(target_os = "windows")]
    {
        let milliseconds = unsafe { GetDoubleClickTime() };
        return Duration::from_millis(milliseconds as u64);
    }

    #[cfg(not(target_os = "windows"))]
    {
        Duration::from_millis(500)
    }
}

#[cfg(target_os = "windows")]
#[link(name = "user32")]
unsafe extern "system" {
    fn GetDoubleClickTime() -> u32;
}

pub fn is_cover_artwork(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    if !matches!(
        extension.to_ascii_lowercase().as_str(),
        "jpg" | "jpeg" | "png"
    ) {
        return false;
    }

    let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return false;
    };

    if stem.to_ascii_lowercase().starts_with("cover") {
        return true;
    }

    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|value| value.to_str())
        .map(|folder_name| stem == folder_name)
        .unwrap_or(false)
}
