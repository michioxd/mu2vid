use anyhow::{Context, Result};
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

const RESUMABLE_UPLOAD_ENDPOINT: &str =
    "https://www.googleapis.com/upload/youtube/v3/videos?uploadType=resumable&part=snippet,status";
const THUMBNAIL_UPLOAD_ENDPOINT: &str =
    "https://www.googleapis.com/upload/youtube/v3/thumbnails/set";
const UPLOAD_CHUNK_SIZE: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
pub struct UploadResponse {
    pub id: Option<String>,
}

#[derive(Debug, Serialize)]
struct VideoInsertRequest<'a> {
    snippet: VideoSnippet<'a>,
    status: VideoStatus<'a>,
}

#[derive(Debug, Serialize)]
struct VideoSnippet<'a> {
    title: &'a str,
    description: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoStatus<'a> {
    privacy_status: &'a str,
    self_declared_made_for_kids: bool,
}

pub async fn upload_video_resumable(
    access_token: &str,
    video_path: &Path,
    title: &str,
    description: &str,
    privacy_status: &str,
    progress_callback: impl Fn(u64, u64) + Send + Sync + 'static,
) -> Result<UploadResponse> {
    let client = reqwest::Client::new();
    let file_size = tokio::fs::metadata(video_path)
        .await
        .with_context(|| format!("Cannot read video file metadata: {}", video_path.display()))?
        .len();

    let upload_url = start_resumable_upload_session(
        &client,
        access_token,
        file_size,
        title,
        description,
        privacy_status,
    )
    .await?;

    upload_file(
        &client,
        &upload_url,
        video_path,
        file_size,
        progress_callback,
    )
    .await
}

pub async fn upload_thumbnail(
    access_token: &str,
    video_id: &str,
    thumbnail_path: &Path,
) -> Result<()> {
    let thumbnail = tokio::fs::read(thumbnail_path)
        .await
        .with_context(|| format!("Cannot read thumbnail: {}", thumbnail_path.display()))?;
    let url = format!("{THUMBNAIL_UPLOAD_ENDPOINT}?videoId={video_id}");

    reqwest::Client::new()
        .post(url)
        .bearer_auth(access_token)
        .header(CONTENT_TYPE, "image/jpeg")
        .header(CONTENT_LENGTH, thumbnail.len() as u64)
        .body(thumbnail)
        .send()
        .await
        .context("Failed to upload YouTube thumbnail")?
        .error_for_status()
        .context("YouTube rejected thumbnail upload")?;

    Ok(())
}

async fn start_resumable_upload_session(
    client: &reqwest::Client,
    access_token: &str,
    file_size: u64,
    title: &str,
    description: &str,
    privacy_status: &str,
) -> Result<String> {
    let body = VideoInsertRequest {
        snippet: VideoSnippet { title, description },
        status: VideoStatus {
            privacy_status,
            self_declared_made_for_kids: false,
        },
    };

    let response = client
        .post(RESUMABLE_UPLOAD_ENDPOINT)
        .bearer_auth(access_token)
        .header(CONTENT_TYPE, "application/json; charset=UTF-8")
        .header("X-Upload-Content-Type", "video/mp4")
        .header("X-Upload-Content-Length", file_size)
        .json(&body)
        .send()
        .await
        .context("Failed to start YouTube resumable upload")?
        .error_for_status()
        .context("YouTube rejected resumable upload session")?;

    response
        .headers()
        .get("Location")
        .context("YouTube resumable upload response is missing Location header")?
        .to_str()
        .context("Invalid YouTube upload Location header")
        .map(str::to_string)
}

async fn upload_file(
    client: &reqwest::Client,
    upload_url: &str,
    video_path: &Path,
    file_size: u64,
    progress_callback: impl Fn(u64, u64) + Send + Sync + 'static,
) -> Result<UploadResponse> {
    let progress_callback = Arc::new(progress_callback);
    let mut file = File::open(video_path)
        .await
        .with_context(|| format!("Cannot open video file: {}", video_path.display()))?;
    let mut uploaded = 0u64;

    if file_size == 0 {
        anyhow::bail!("Video file is empty: {}", video_path.display());
    }

    loop {
        let remaining = file_size.saturating_sub(uploaded);
        let chunk_len = remaining.min(UPLOAD_CHUNK_SIZE as u64) as usize;
        let mut chunk = vec![0; chunk_len];
        file.read_exact(&mut chunk)
            .await
            .with_context(|| format!("Cannot read video file: {}", video_path.display()))?;

        let chunk_start = uploaded;
        let chunk_end = uploaded + chunk_len as u64 - 1;
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_LENGTH, HeaderValue::from(chunk_len as u64));
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
        headers.insert(
            reqwest::header::CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {chunk_start}-{chunk_end}/{file_size}"))?,
        );

        let response = client
            .put(upload_url)
            .headers(headers)
            .body(chunk)
            .send()
            .await
            .context("Failed to upload video chunk to YouTube")?;
        let status = response.status();

        if status == reqwest::StatusCode::PERMANENT_REDIRECT {
            uploaded = uploaded_from_range_header(response.headers())
                .unwrap_or(chunk_end.saturating_add(1));
            progress_callback(uploaded.min(file_size), file_size);
            continue;
        }

        let response = response
            .error_for_status()
            .context("YouTube rejected video upload")?;
        uploaded = file_size;
        progress_callback(uploaded, file_size);
        return response
            .json::<UploadResponse>()
            .await
            .context("Failed to parse YouTube upload response");
    }
}

fn uploaded_from_range_header(headers: &HeaderMap) -> Option<u64> {
    let range = headers.get("Range")?.to_str().ok()?;
    range
        .strip_prefix("bytes=0-")?
        .parse::<u64>()
        .ok()
        .map(|value| value + 1)
}
