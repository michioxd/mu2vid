use crate::ui::new_queue::QueueItemDraft;
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
    pub audio_quality: String,
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
            audio_quality: item.audio_quality.clone(),
        }
    }
}

impl From<ProjectAlbum> for QueueItemDraft {
    fn from(album: ProjectAlbum) -> Self {
        Self {
            album_path: album.album_path,
            artwork_path: PathBuf::from(album.artwork_path),
            title: album.title,
            video_quality: album.video_quality,
            audio_quality: album.audio_quality,
        }
    }
}
