use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;

const CACHE_DIR: &str = "saumfar";
const SERVICE_FEED_FILE: &str = "service_feed.xml";
const MAX_AGE: Duration = Duration::from_secs(3600);

fn cache_dir() -> Result<PathBuf> {
    let base = dirs::cache_dir().context("could not determine cache directory")?;
    Ok(base.join(CACHE_DIR))
}

pub async fn read_cached_feed() -> Option<String> {
    let path = cache_dir().ok()?.join(SERVICE_FEED_FILE);
    let meta = fs::metadata(&path).await.ok()?;
    let age = meta.modified().ok()?.elapsed().unwrap_or(Duration::MAX);
    if age > MAX_AGE {
        return None;
    }
    fs::read_to_string(&path).await.ok()
}

pub async fn write_cached_feed(xml: &str) -> Result<()> {
    let dir = cache_dir()?;
    fs::create_dir_all(&dir).await?;
    fs::write(dir.join(SERVICE_FEED_FILE), xml).await?;
    Ok(())
}

pub async fn cache_age() -> Option<Duration> {
    let path = cache_dir().ok()?.join(SERVICE_FEED_FILE);
    let meta = fs::metadata(&path).await.ok()?;
    meta.modified().ok()?.elapsed().ok()
}

pub async fn invalidate() -> Result<()> {
    let path = cache_dir()?.join(SERVICE_FEED_FILE);
    if fs::try_exists(&path).await.unwrap_or(false) {
        fs::remove_file(&path).await?;
    }
    Ok(())
}
