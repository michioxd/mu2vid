use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ffmpeg_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self { ffmpeg_path: None }
    }
}
