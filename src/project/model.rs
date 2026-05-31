use crate::ui::new_queue::{
    DEFAULT_AUDIO_BITRATE_KBPS, DEFAULT_AUDIO_CODEC, ORIGINAL_AUDIO_CODEC, QueueItemDraft,
    QueueRenderStatus, clamp_audio_bitrate,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const PROJECT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub mu2vid_version: String,
    pub version: u32,
    pub title: String,
    pub work_dir: String,
    #[serde(default)]
    pub albums: Vec<ProjectAlbum>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAlbum {
    pub album_path: String,
    pub artwork_path: String,
    pub title: String,
    pub video_quality: String,
    #[serde(default = "default_audio_codec")]
    pub audio_codec: String,
    #[serde(default = "default_audio_bitrate_kbps")]
    pub audio_bitrate_kbps: u32,
    #[serde(default = "default_render_status")]
    pub render_status: String,
    #[serde(default)]
    pub skip_render: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_quality: Option<String>,
}

impl ProjectFile {
    pub fn new(title: String, work_dir: String, albums: Vec<ProjectAlbum>) -> Self {
        Self {
            mu2vid_version: env!("CARGO_PKG_VERSION").to_string(),
            version: PROJECT_VERSION,
            title,
            work_dir,
            albums,
        }
    }
}

impl From<&QueueItemDraft> for ProjectAlbum {
    fn from(item: &QueueItemDraft) -> Self {
        Self {
            album_path: item.album_path.clone(),
            artwork_path: item.artwork_path.to_string_lossy().to_string(),
            title: item.title.clone(),
            video_quality: item.video_quality.clone(),
            audio_codec: item.audio_codec.clone(),
            audio_bitrate_kbps: item.audio_bitrate_kbps,
            render_status: item.render_status.as_str().to_string(),
            skip_render: item.skip_render,
            audio_quality: None,
        }
    }
}

impl From<ProjectAlbum> for QueueItemDraft {
    fn from(album: ProjectAlbum) -> Self {
        let (audio_codec, audio_bitrate_kbps) = normalized_audio_settings(&album);

        Self {
            album_path: album.album_path,
            artwork_path: PathBuf::from(album.artwork_path),
            title: album.title,
            video_quality: album.video_quality,
            audio_codec,
            audio_bitrate_kbps,
            render_status: QueueRenderStatus::from(album.render_status.as_str()),
            skip_render: album.skip_render,
        }
    }
}

fn default_audio_codec() -> String {
    DEFAULT_AUDIO_CODEC.to_string()
}

fn default_audio_bitrate_kbps() -> u32 {
    DEFAULT_AUDIO_BITRATE_KBPS
}

fn default_render_status() -> String {
    QueueRenderStatus::Waiting.as_str().to_string()
}

fn normalized_audio_settings(album: &ProjectAlbum) -> (String, u32) {
    if let Some(settings) = album
        .audio_quality
        .as_deref()
        .and_then(parse_legacy_audio_quality)
    {
        return settings;
    }

    if !album.audio_codec.is_empty() {
        return (
            album.audio_codec.to_lowercase(),
            clamp_audio_bitrate(album.audio_bitrate_kbps),
        );
    }

    (DEFAULT_AUDIO_CODEC.to_string(), DEFAULT_AUDIO_BITRATE_KBPS)
}

fn parse_legacy_audio_quality(value: &str) -> Option<(String, u32)> {
    let value = value.trim().to_lowercase();
    if value == "original" {
        return Some((ORIGINAL_AUDIO_CODEC.to_string(), DEFAULT_AUDIO_BITRATE_KBPS));
    }

    let bitrate_end = value.find("kbps")?;
    let bitrate = value[..bitrate_end].trim().parse::<u32>().ok()?;
    let codec_start = value.find('(')? + 1;
    let codec_end = value[codec_start..].find(')')? + codec_start;
    let codec = value[codec_start..codec_end].trim();
    if codec.is_empty() {
        None
    } else {
        Some((codec.to_string(), clamp_audio_bitrate(bitrate)))
    }
}
