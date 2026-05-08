use anyhow::{Context, Result};

pub const SERVICE_FEED_URL: &str = "https://nedlasting.geonorge.no/geonorge/Tjenestefeed.xml";

const ATOM_NS: &str = "http://www.w3.org/2005/Atom";

#[derive(Debug, Clone)]
pub struct Dataset {
    pub title: String,
    pub summary: String,
    pub owner: String,
    pub updated: String,
    pub crs: String,
    pub feed_url: String,
}

#[derive(Debug, Clone)]
pub struct DownloadEntry {
    pub title: String,
    pub format: String,
    pub url: String,
    pub updated: String,
}

pub async fn fetch_service_feed(client: &reqwest::Client) -> Result<Vec<Dataset>> {
    let body = client
        .get(SERVICE_FEED_URL)
        .send()
        .await
        .context("failed to fetch service feed")?
        .text()
        .await
        .context("failed to read service feed body")?;
    parse_service_feed(&body)
}

pub fn parse_service_feed(xml: &str) -> Result<Vec<Dataset>> {
    let doc = roxmltree::Document::parse(xml).context("failed to parse service feed XML")?;
    let root = doc.root_element();

    let mut datasets = Vec::new();
    for entry in root
        .children()
        .filter(|n| n.has_tag_name((ATOM_NS, "entry")))
    {
        let title = child_text(&entry, "title").unwrap_or_default();
        let summary = child_text(&entry, "summary").unwrap_or_default();
        let owner = child_text(&entry, "rights").unwrap_or_default();
        let updated = child_text(&entry, "updated").unwrap_or_default();

        let crs = entry
            .children()
            .filter(|n| n.has_tag_name((ATOM_NS, "category")))
            .find_map(|n| {
                let term = n.attribute("term")?;
                term.starts_with("EPSG:").then(|| term.to_string())
            })
            .unwrap_or_default();

        let feed_url = entry
            .children()
            .filter(|n| n.has_tag_name((ATOM_NS, "link")))
            .find(|n| n.attribute("type") == Some("application/atom+xml"))
            .and_then(|n| n.attribute("href"))
            .unwrap_or("")
            .to_string();

        datasets.push(Dataset {
            title,
            summary,
            owner,
            updated,
            crs,
            feed_url,
        });
    }

    Ok(datasets)
}

pub async fn fetch_dataset_feed(client: &reqwest::Client, url: &str) -> Result<Vec<DownloadEntry>> {
    let body = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to fetch dataset feed: {url}"))?
        .text()
        .await?;
    parse_dataset_feed(&body)
}

pub fn parse_dataset_feed(xml: &str) -> Result<Vec<DownloadEntry>> {
    let doc = roxmltree::Document::parse(xml).context("failed to parse dataset feed XML")?;
    let root = doc.root_element();

    let mut entries = Vec::new();
    for entry in root
        .children()
        .filter(|n| n.has_tag_name((ATOM_NS, "entry")))
    {
        let title = child_text(&entry, "title").unwrap_or_default();
        let updated = child_text(&entry, "updated").unwrap_or_default();

        let link = entry
            .children()
            .filter(|n| n.has_tag_name((ATOM_NS, "link")))
            .find(|n| n.attribute("rel") == Some("alternate"))
            .or_else(|| {
                entry
                    .children()
                    .find(|n| n.has_tag_name((ATOM_NS, "link")) && n.attribute("href").is_some())
            });

        let url = link
            .and_then(|n| n.attribute("href"))
            .unwrap_or("")
            .to_string();
        let format = link
            .and_then(|n| n.attribute("type"))
            .unwrap_or("")
            .to_string();

        entries.push(DownloadEntry {
            title,
            format,
            url,
            updated,
        });
    }

    Ok(entries)
}

fn child_text(node: &roxmltree::Node, tag: &str) -> Option<String> {
    node.children()
        .find(|n| n.has_tag_name((ATOM_NS, "tag")) || n.has_tag_name(tag))
        .and_then(|n| n.text())
        .map(|s| s.trim().to_string())
}
