use serde::Deserialize;

// ──────────────────────────────────────
// FxTwitter API response types
// ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct FxApiResponse {
    pub code: i32,
    pub message: String,
    pub tweet: Option<FxTweet>,
}

#[derive(Debug, Deserialize)]
pub struct FxTweet {
    pub text: String,
    pub author: FxAuthor,
    pub media: Option<FxMedia>,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    /// Thread: if this tweet is part of a thread, the API may include it
    pub thread: Option<FxThread>,
    /// Quoted tweet
    pub quote: Option<Box<FxTweet>>,
    /// X Article (long-form content)
    pub article: Option<FxArticle>,
}

#[derive(Debug, Deserialize)]
pub struct FxAuthor {
    pub name: String,
    pub screen_name: String,
}

#[derive(Debug, Deserialize)]
pub struct FxMedia {
    pub photos: Option<Vec<FxPhoto>>,
    pub videos: Option<Vec<FxVideo>>,
}

#[derive(Debug, Deserialize)]
pub struct FxPhoto {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct FxVideo {
    pub url: String,
    #[serde(rename = "thumbnail_url")]
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FxThread {
    pub tweets: Vec<FxTweet>,
}

// ──────────────────────────────────────
// FxTwitter Article types
// ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct FxArticle {
    pub title: Option<String>,
    pub preview_text: Option<String>,
    pub cover_media: Option<FxCoverMedia>,
    pub content: Option<FxArticleContent>,
}

#[derive(Debug, Deserialize)]
pub struct FxCoverMedia {
    pub media_info: Option<FxCoverMediaInfo>,
}

#[derive(Debug, Deserialize)]
pub struct FxCoverMediaInfo {
    pub original_img_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FxArticleContent {
    pub blocks: Vec<FxBlock>,
    #[serde(rename = "entityMap", default)]
    pub entity_map: Vec<FxEntityMapEntry>,
}

#[derive(Debug, Deserialize)]
pub struct FxBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: String,
    #[serde(rename = "inlineStyleRanges", default)]
    #[allow(dead_code)]
    pub inline_style_ranges: Vec<FxInlineStyle>,
    #[serde(rename = "entityRanges", default)]
    pub entity_ranges: Vec<FxEntityRange>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct FxInlineStyle {
    pub offset: usize,
    pub length: usize,
    pub style: String,
}

#[derive(Debug, Deserialize)]
pub struct FxEntityRange {
    pub key: serde_json::Value,
    #[allow(dead_code)]
    pub offset: usize,
    #[allow(dead_code)]
    pub length: usize,
}

#[derive(Debug, Deserialize)]
pub struct FxEntityMapEntry {
    pub key: String,
    pub value: FxEntityValue,
}

#[derive(Debug, Deserialize)]
pub struct FxEntityValue {
    pub data: Option<FxEntityData>,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub entity_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FxEntityData {
    pub markdown: Option<String>,
}

// ──────────────────────────────────────
// Internal structured types
// ──────────────────────────────────────

/// Normalized tweet data after fetching
#[derive(Debug)]
pub struct TweetData {
    pub author: Author,
    pub posts: Vec<Post>,
    pub source_url: String,
    pub article_title: Option<String>,
}

#[derive(Debug)]
pub struct Author {
    pub name: String,
    pub screen_name: String,
}

#[derive(Debug)]
pub struct Post {
    pub text: String,
    pub images: Vec<String>, // image URLs
    pub quoted: Option<QuotedPost>,
}

#[derive(Debug)]
pub struct QuotedPost {
    pub author_name: String,
    pub text: String,
}

// ──────────────────────────────────────
// Article output types
// ──────────────────────────────────────

/// The translated & rewritten article, ready for rendering
#[derive(Debug)]
pub struct Article {
    pub title: String,
    pub summary: String, // one-line summary / subtitle
    pub sections: Vec<Section>,
}

#[derive(Debug)]
pub struct Section {
    pub heading: Option<String>,
    pub body: String, // may contain multiple paragraphs separated by \n\n
}
