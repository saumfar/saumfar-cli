use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::path::Path;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct DownloadState {
    pub filename: String,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub done: bool,
    pub error: Option<String>,
}

pub type SharedDownloads = Arc<Mutex<Vec<DownloadState>>>;

pub fn new_shared_downloads() -> SharedDownloads {
    Arc::new(Mutex::new(Vec::new()))
}

pub async fn download_parallel(
    client: &reqwest::Client,
    urls: Vec<String>,
    output_dir: &Path,
    state: SharedDownloads,
) {
    {
        let mut downloads = state.lock().await;
        for url in &urls {
            downloads.push(DownloadState {
                filename: filename_from_url(url),
                bytes_downloaded: 0,
                total_bytes: None,
                done: false,
                error: None,
            });
        }
    }

    let mut handles = Vec::new();
    for (i, url) in urls.into_iter().enumerate() {
        let client = client.clone();
        let dir = output_dir.to_path_buf();
        let state = state.clone();
        handles.push(tokio::spawn(async move {
            download_one(&client, &url, &dir, &state, i).await;
        }));
    }

    for h in handles {
        let _ = h.await;
    }
}

async fn download_one(
    client: &reqwest::Client,
    url: &str,
    output_dir: &Path,
    state: &SharedDownloads,
    index: usize,
) {
    let result = download_streaming(client, url, output_dir, state, index).await;
    if let Err(e) = result {
        let mut downloads = state.lock().await;
        if let Some(dl) = downloads.get_mut(index) {
            dl.error = Some(format!("{e:#}"));
            dl.done = true;
        }
    }
}

async fn download_streaming(
    client: &reqwest::Client,
    url: &str,
    output_dir: &Path,
    state: &SharedDownloads,
    index: usize,
) -> Result<()> {
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to request {url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP error for {url}"))?;

    let total_bytes = resp.content_length();
    {
        let mut downloads = state.lock().await;
        if let Some(dl) = downloads.get_mut(index) {
            dl.total_bytes = total_bytes;
        }
    }

    let filename = filename_from_url(url);
    let filepath = output_dir.join(&filename);
    let mut file = tokio::fs::File::create(&filepath)
        .await
        .with_context(|| format!("failed to create {}", filepath.display()))?;

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("error reading response stream")?;
        file.write_all(&chunk).await?;
        let len = chunk.len() as u64;
        let mut downloads = state.lock().await;
        if let Some(dl) = downloads.get_mut(index) {
            dl.bytes_downloaded += len;
        }
    }

    file.flush().await?;

    let mut downloads = state.lock().await;
    if let Some(dl) = downloads.get_mut(index) {
        dl.done = true;
    }

    Ok(())
}

fn filename_from_url(url: &str) -> String {
    url.rsplit('/')
        .next()
        .unwrap_or("download")
        .split('?')
        .next()
        .unwrap_or("download")
        .to_string()
}
