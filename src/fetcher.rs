use anyhow::{bail, Context, Result};
use std::collections::{HashMap, HashSet};

use crate::error::Tweet2WxError;
use crate::types::*;

/// Parse tweet URL to extract screen_name and status_id
/// Supports:
///   https://x.com/user/status/123456789
///   https://twitter.com/user/status/123456789
fn parse_tweet_url(url: &str) -> Result<(String, String)> {
    let url = url.trim().trim_end_matches('/');

    // Extract path segments
    let parsed = url
        .strip_prefix("https://x.com/")
        .or_else(|| url.strip_prefix("https://twitter.com/"))
        .or_else(|| url.strip_prefix("https://www.x.com/"))
        .or_else(|| url.strip_prefix("https://www.twitter.com/"))
        .ok_or_else(|| Tweet2WxError::InvalidUrl(url.to_string()))?;

    let parts: Vec<&str> = parsed.split('/').collect();

    // Expected: ["user", "status", "123456789"] possibly with query params
    if parts.len() < 3 || parts[1] != "status" {
        bail!(Tweet2WxError::InvalidUrl(url.to_string()));
    }

    let screen_name = parts[0].to_string();
    let status_id = parts[2].split('?').next().unwrap_or(parts[2]).to_string();

    Ok((screen_name, status_id))
}

/// Fetch tweet data via FxTwitter API
pub async fn fetch_tweet(url: &str) -> Result<TweetData> {
    let (screen_name, status_id) = parse_tweet_url(url)?;

    let api_url = format!("https://api.fxtwitter.com/{screen_name}/status/{status_id}");

    let client = reqwest::Client::builder()
        .user_agent("tweet2wx/0.1")
        .build()?;

    let resp: FxApiResponse = client
        .get(&api_url)
        .send()
        .await
        .with_context(|| "Failed to connect to FxTwitter API")?
        .json()
        .await
        .with_context(|| "Failed to parse FxTwitter response")?;

    if resp.code != 200 {
        bail!(Tweet2WxError::FetchFailed(resp.message));
    }

    let tweet = resp
        .tweet
        .ok_or_else(|| Tweet2WxError::FetchFailed("No tweet data in response".into()))?;

    Ok(normalize_tweet(&tweet, url))
}

/// Convert FxTwitter response into our internal TweetData
fn normalize_tweet(tweet: &FxTweet, source_url: &str) -> TweetData {
    let author = Author {
        name: tweet.author.name.clone(),
        screen_name: tweet.author.screen_name.clone(),
    };

    // Check if this is an Article type
    if let Some(ref article) = tweet.article {
        let (text, images) = extract_article_content(article);

        let article_title = article.title.clone();

        return TweetData {
            author,
            posts: vec![Post {
                text,
                images,
                quoted: None,
            }],
            source_url: source_url.to_string(),
            article_title,
        };
    }

    let mut posts = vec![normalize_post(tweet)];

    // If this is a thread, append the rest
    if let Some(ref thread) = tweet.thread {
        for t in &thread.tweets {
            posts.push(normalize_post(t));
        }
    }

    TweetData {
        author,
        posts,
        source_url: source_url.to_string(),
        article_title: None,
    }
}

/// Extract article body while preserving image positions with placeholders.
fn extract_article_content(article: &FxArticle) -> (String, Vec<String>) {
    let mut parts: Vec<String> = Vec::new();
    let mut images = Vec::new();
    let mut seen = HashSet::new();
    let mut ordered_counter: u32 = 0;
    let mut prev_was_list = false;
    let media_by_id: HashMap<&str, &str> = article
        .media_entities
        .iter()
        .filter_map(|entity| {
            entity
                .media_info
                .as_ref()
                .and_then(|info| info.original_img_url.as_deref())
                .map(|url| (entity.media_id.as_str(), url))
        })
        .collect();

    if let Some(url) = article
        .cover_media
        .as_ref()
        .and_then(|c| c.media_info.as_ref())
        .and_then(|m| m.original_img_url.clone())
    {
        push_image_placeholder(&mut parts, &mut images, &mut seen, url);
    }

    let content = match article.content.as_ref() {
        Some(c) => c,
        None => {
            if let Some(preview) = article.preview_text.as_ref() {
                if !preview.trim().is_empty() {
                    parts.push(preview.clone());
                }
            }
            return (parts.join("\n\n"), images);
        }
    };

    for block in &content.blocks {
        let is_list =
            block.block_type == "ordered-list-item" || block.block_type == "unordered-list-item";

        match block.block_type.as_str() {
            "unstyled" => {
                ordered_counter = 0;
                prev_was_list = false;
                if !block.text.trim().is_empty() {
                    parts.push(block.text.clone());
                }
            }
            "header-two" | "header-three" => {
                ordered_counter = 0;
                prev_was_list = false;
                parts.push(format!("## {}", block.text));
            }
            "header-one" => {
                ordered_counter = 0;
                prev_was_list = false;
                parts.push(format!("# {}", block.text));
            }
            "ordered-list-item" => {
                if !prev_was_list {
                    ordered_counter = 0;
                }
                ordered_counter += 1;
                let item = format!("{}. {}", ordered_counter, block.text);
                // Merge consecutive list items with single newline
                if prev_was_list {
                    if let Some(last) = parts.last_mut() {
                        last.push('\n');
                        last.push_str(&item);
                    }
                } else {
                    parts.push(item);
                }
                prev_was_list = true;
                continue;
            }
            "unordered-list-item" => {
                ordered_counter = 0;
                let item = format!("- {}", block.text);
                if prev_was_list {
                    if let Some(last) = parts.last_mut() {
                        last.push('\n');
                        last.push_str(&item);
                    }
                } else {
                    parts.push(item);
                }
                prev_was_list = true;
                continue;
            }
            "atomic" => {
                ordered_counter = 0;
                prev_was_list = false;
                for range in &block.entity_ranges {
                    let key_str = match &range.key {
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    if let Some(entry) = content.entity_map.iter().find(|e| e.key == key_str) {
                        if let Some(data) = entry.value.data.as_ref() {
                            for media_item in &data.media_items {
                                if let Some(url) = media_by_id.get(media_item.media_id.as_str()) {
                                    push_image_placeholder(
                                        &mut parts,
                                        &mut images,
                                        &mut seen,
                                        (*url).to_string(),
                                    );
                                }
                            }
                        }

                        if let Some(md) =
                            entry.value.data.as_ref().and_then(|d| d.markdown.as_ref())
                        {
                            parts.push(md.clone());
                        }
                    }
                }
            }
            _ => {
                ordered_counter = 0;
                prev_was_list = false;
                if !block.text.trim().is_empty() {
                    parts.push(block.text.clone());
                }
            }
        }

        if !is_list {
            prev_was_list = false;
        }
    }

    (parts.join("\n\n"), images)
}

fn push_image_placeholder(
    parts: &mut Vec<String>,
    images: &mut Vec<String>,
    seen: &mut HashSet<String>,
    url: String,
) {
    if seen.insert(url.clone()) {
        images.push(url);
        parts.push(format!("[[IMAGE:{}]]", images.len()));
    }
}

fn normalize_post(tweet: &FxTweet) -> Post {
    let images = tweet
        .media
        .as_ref()
        .and_then(|m| m.photos.as_ref())
        .map(|photos| photos.iter().map(|p| p.url.clone()).collect())
        .unwrap_or_default();

    let quoted = tweet.quote.as_ref().map(|q| QuotedPost {
        author_name: format!("@{}", q.author.screen_name),
        text: q.text.clone(),
    });

    Post {
        text: tweet.text.clone(),
        images,
        quoted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_tweet_url() {
        let (user, id) = parse_tweet_url("https://x.com/karpathy/status/1234567890").unwrap();
        assert_eq!(user, "karpathy");
        assert_eq!(id, "1234567890");

        let (user, id) = parse_tweet_url("https://twitter.com/elonmusk/status/9876?s=20").unwrap();
        assert_eq!(user, "elonmusk");
        assert_eq!(id, "9876");
    }

    #[test]
    fn test_invalid_url() {
        assert!(parse_tweet_url("https://google.com").is_err());
        assert!(parse_tweet_url("https://x.com/user").is_err());
    }

    #[test]
    fn test_extracts_article_body_images_from_media_entities_in_order() {
        let tweet: FxTweet = serde_json::from_value(json!({
            "text": "",
            "author": {
                "name": "Alice",
                "screen_name": "alice"
            },
            "article": {
                "title": "Test Article",
                "preview_text": "Preview",
                "cover_media": {
                    "media_info": {
                        "original_img_url": "https://pbs.twimg.com/media/cover.jpg"
                    }
                },
                "content": {
                    "blocks": [
                        {
                            "type": "unstyled",
                            "text": "intro",
                            "entityRanges": [],
                            "inlineStyleRanges": []
                        },
                        {
                            "type": "atomic",
                            "text": " ",
                            "entityRanges": [{ "key": 0, "offset": 0, "length": 1 }],
                            "inlineStyleRanges": []
                        },
                        {
                            "type": "atomic",
                            "text": " ",
                            "entityRanges": [{ "key": 1, "offset": 0, "length": 1 }],
                            "inlineStyleRanges": []
                        }
                    ],
                    "entityMap": [
                        {
                            "key": "0",
                            "value": {
                                "type": "MEDIA",
                                "data": {
                                    "mediaItems": [{ "mediaId": "m1" }]
                                }
                            }
                        },
                        {
                            "key": "1",
                            "value": {
                                "type": "MEDIA",
                                "data": {
                                    "mediaItems": [{ "mediaId": "m2" }]
                                }
                            }
                        }
                    ]
                },
                "media_entities": [
                    {
                        "media_id": "m1",
                        "media_info": {
                            "original_img_url": "https://pbs.twimg.com/media/body-1.jpg"
                        }
                    },
                    {
                        "media_id": "m2",
                        "media_info": {
                            "original_img_url": "https://pbs.twimg.com/media/body-2.png"
                        }
                    }
                ]
            }
        }))
        .unwrap();

        let normalized = normalize_tweet(&tweet, "https://x.com/alice/status/1");
        let body = &normalized.posts[0].text;

        assert!(body.contains("[[IMAGE:1]]"));
        assert!(body.contains("[[IMAGE:2]]"));
        assert!(body.contains("[[IMAGE:3]]"));
        assert!(body.find("[[IMAGE:1]]").unwrap() < body.find("intro").unwrap());
        assert!(body.find("[[IMAGE:2]]").unwrap() > body.find("intro").unwrap());
        assert!(body.find("[[IMAGE:3]]").unwrap() > body.find("[[IMAGE:2]]").unwrap());
        assert_eq!(
            normalized.posts[0].images,
            vec![
                "https://pbs.twimg.com/media/cover.jpg".to_string(),
                "https://pbs.twimg.com/media/body-1.jpg".to_string(),
                "https://pbs.twimg.com/media/body-2.png".to_string(),
            ]
        );
    }
}
