use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub ffmpeg_path: Option<String>,
    pub window: WindowConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub maximized: bool,
    pub main_splitter_sash_position: Option<i32>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ffmpeg_path: None,
            window: WindowConfig::default(),
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            width: None,
            height: None,
            maximized: false,
            main_splitter_sash_position: None,
        }
    }
}
