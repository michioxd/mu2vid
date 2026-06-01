use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub ffmpeg_path: Option<String>,
    pub appearance: AppearanceConfig,
    pub encoder: EncoderConfig,
    pub youtube: YoutubeConfig,
    pub window: WindowConfig,
    pub recent_projects: Vec<String>,
    pub last_project_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppearanceConfig {
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EncoderConfig {
    pub video_encoder: Option<String>,
    pub default_video_quality: String,
    pub default_audio_encoder: String,
    pub default_audio_bitrate_kbps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct YoutubeConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub upload_visibility: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
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
            appearance: AppearanceConfig::default(),
            encoder: EncoderConfig::default(),
            youtube: YoutubeConfig::default(),
            window: WindowConfig::default(),
            recent_projects: Vec::new(),
            last_project_path: None,
        }
    }
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self::System
    }
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            video_encoder: None,
            default_video_quality: "1080p".to_string(),
            default_audio_encoder: "aac".to_string(),
            default_audio_bitrate_kbps: 320,
        }
    }
}

impl Default for YoutubeConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: "http://localhost".to_string(),
            upload_visibility: "public".to_string(),
            access_token: None,
            refresh_token: None,
            expires_at: None,
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
