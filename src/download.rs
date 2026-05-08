use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::path::Path;
use tokio::io::AsyncWriteExt;

pub struct DownloadProgress {
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub filename: String,
}

pub async fn download_file(
    client: &reqwest::Client,
    url: &str,
    output_dir: &Path,
) -> Result<DownloadProgress> {
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to request {url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP error for {url}"))?;

    let total_bytes = resp.content_length();

    let filename = url
        .rsplit('/')
        .next()
        .unwrap_or("download")
        .split('?')
        .next()
        .unwrap_or("download")
        .to_string();

    let filepath = output_dir.join(&filename);
    let mut file = tokio::fs::File::create(&filepath)
        .await
        .with_context(|| format!("failed to create {}", filepath.display()))?;

    let mut stream = resp.bytes_stream();
    let mut bytes_downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("error reading response stream")?;
        file.write_all(&chunk).await?;
        bytes_downloaded += chunk.len() as u64;
    }

    file.flush().await?;

    Ok(DownloadProgress {
        bytes_downloaded,
        total_bytes,
        filename,
    })
}
