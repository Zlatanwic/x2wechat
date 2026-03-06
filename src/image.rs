use base64::Engine;
use futures::stream::{self, StreamExt};

const MAX_IMAGE_SIZE: usize = 5 * 1024 * 1024; // 5MB
const MAX_CONCURRENT: usize = 4;

pub struct EmbeddedImage {
    pub original_url: String,
    pub base64_data: String,
    pub mime_type: String,
}

/// Batch download images and convert to base64.
/// Failed downloads are skipped with a warning printed to stderr.
pub async fn download_and_embed(urls: &[String]) -> Vec<EmbeddedImage> {
    let client = reqwest::Client::new();

    let results: Vec<Option<EmbeddedImage>> = stream::iter(urls)
        .map(|url| {
            let client = &client;
            async move {
                match download_single(client, url).await {
                    Ok(img) => Some(img),
                    Err(e) => {
                        eprintln!("⚠️  Failed to download image {}: {}", url, e);
                        None
                    }
                }
            }
        })
        .buffer_unordered(MAX_CONCURRENT)
        .collect()
        .await;

    results.into_iter().flatten().collect()
}

async fn download_single(client: &reqwest::Client, url: &str) -> anyhow::Result<EmbeddedImage> {
    let resp = client.get(url).send().await?.error_for_status()?;

    let mime_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or(s).trim().to_string())
        .unwrap_or_else(|| mime_from_url(url));

    let bytes = resp.bytes().await?;

    if bytes.len() > MAX_IMAGE_SIZE {
        anyhow::bail!(
            "Image too large ({:.1} MB, limit is 5 MB)",
            bytes.len() as f64 / 1024.0 / 1024.0
        );
    }

    let base64_data = base64::engine::general_purpose::STANDARD.encode(&bytes);

    Ok(EmbeddedImage {
        original_url: url.to_string(),
        base64_data,
        mime_type,
    })
}

fn mime_from_url(url: &str) -> String {
    let path = url.split('?').next().unwrap_or(url);
    if path.ends_with(".png") {
        "image/png".into()
    } else if path.ends_with(".gif") {
        "image/gif".into()
    } else if path.ends_with(".webp") {
        "image/webp".into()
    } else {
        "image/jpeg".into()
    }
}
