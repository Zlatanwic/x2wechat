use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::Args;
use crate::config::Config;
use crate::types::*;

// ──────────────────────────────────────
// DeepSeek API request/response types (OpenAI-compatible)
// ──────────────────────────────────────

#[derive(Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ApiResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

// ──────────────────────────────────────
// Core translation logic
// ──────────────────────────────────────

pub async fn translate_and_rewrite(
    config: &Config,
    tweet: &TweetData,
    args: &Args,
) -> Result<Article> {
    let max_tokens = validate_max_tokens(args.max_tokens)?;
    let system_prompt = build_system_prompt(&args.style);
    let user_prompt = build_user_prompt(tweet);

    let request = ApiRequest {
        model: args.model.clone(),
        max_tokens,
        messages: vec![
            Message {
                role: "system".into(),
                content: system_prompt,
            },
            Message {
                role: "user".into(),
                content: user_prompt,
            },
        ],
    };

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.deepseek.com/chat/completions")
        .header(
            "Authorization",
            format!("Bearer {}", config.deepseek_api_key),
        )
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await
        .with_context(|| "Failed to call DeepSeek API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("DeepSeek API returned {status}: {body}");
    }

    let api_resp: ApiResponse = resp
        .json()
        .await
        .with_context(|| "Failed to parse DeepSeek API response")?;

    let choice = api_resp
        .choices
        .first()
        .ok_or_else(|| anyhow::anyhow!("DeepSeek API returned no choices"))?;

    validate_finish_reason(choice.finish_reason.as_deref(), max_tokens)?;

    let text = choice.message.content.clone();

    parse_article_response(&text)
}

fn validate_max_tokens(max_tokens: u32) -> Result<u32> {
    if !(1..=8192).contains(&max_tokens) {
        bail!(
            "Invalid --max-tokens value {max_tokens}. DeepSeek only accepts values in [1, 8192]."
        );
    }

    Ok(max_tokens)
}

fn validate_finish_reason(finish_reason: Option<&str>, max_tokens: u32) -> Result<()> {
    if matches!(finish_reason, Some("length")) {
        bail!(
            "LLM output was truncated because it hit max_tokens={max_tokens}. \
Try rerunning with a higher --max-tokens value."
        );
    }

    Ok(())
}

fn build_system_prompt(style: &str) -> String {
    let style_desc = match style {
        "casual" => "轻松活泼，像朋友聊天一样",
        "technical" => "专业严谨，保留技术术语的英文原文并附中文解释",
        _ => "信息丰富、条理清晰、适合大众阅读",
    };

    format!(
        r#"你是一个专业的翻译，负责将英文内容忠实翻译为简体中文。

## 任务
将用户提供的 X/Twitter 推文内容翻译为简体中文，保持原文的结构和内容不变。

## 风格要求
{style_desc}

## 输出格式
请严格按以下 XML 格式输出：

<article>
<title>文章标题</title>
<summary>一句话摘要</summary>
<section>
<heading>小标题（如果需要）</heading>
<body>正文段落，段落之间用空行分隔</body>
</section>
<section>
<body>没有小标题的段落也可以</body>
</section>
</article>

## 注意事项
- 忠实翻译原文内容，不要增删改写，保持原文的段落结构和顺序
- 原文中的代码块（```...```）和行内代码（`...`）必须原样保留，不要翻译代码内容
- 原文中如果出现 `[[IMAGE:n]]` 这样的图片占位符，必须原样保留在对应位置，不要删除、移动、改写
- 保留专业术语的英文原文（如 LLM, RLHF, transformer 等）
- 如有引用推文（quote），保持引用结构
- 标题翻译原文标题即可，不要另起标题"#
    )
}

fn build_user_prompt(tweet: &TweetData) -> String {
    let mut content = String::new();

    if let Some(ref title) = tweet.article_title {
        content.push_str(&format!(
            "这是一篇 X 平台的长文章（Article），原标题为：\"{}\"\n\n",
            title
        ));
    }

    content.push_str(&format!(
        "以下是 @{} ({}) 的推文内容，请改写为公众号文章：\n\n",
        tweet.author.screen_name, tweet.author.name
    ));

    content.push_str(
        "注意：如果正文里出现 `[[IMAGE:n]]`，那是图片位置标记，必须在输出中原样保留在同一位置。\n\n",
    );

    for (i, post) in tweet.posts.iter().enumerate() {
        if tweet.posts.len() > 1 {
            content.push_str(&format!("--- 第 {} 条 ---\n", i + 1));
        }
        content.push_str(&post.text);
        content.push('\n');

        if let Some(ref quoted) = post.quoted {
            content.push_str(&format!(
                "\n[引用 {} 的推文]: {}\n",
                quoted.author_name, quoted.text
            ));
        }
        content.push('\n');
    }

    content
}

/// Parse the XML-formatted article response from the LLM
fn parse_article_response(text: &str) -> Result<Article> {
    // Simple XML-like parsing (not a full XML parser)
    let title = extract_tag(text, "title").unwrap_or_else(|| "无标题".into());
    let summary = extract_tag(text, "summary").unwrap_or_default();

    let mut sections = Vec::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("<section>") {
        if let Some(end) = remaining[start..].find("</section>") {
            let section_content = &remaining[start + 9..start + end];
            let heading = extract_tag(section_content, "heading");
            let body = extract_tag(section_content, "body").unwrap_or_default();

            sections.push(Section { heading, body });
            remaining = &remaining[start + end + 10..];
        } else {
            break;
        }
    }

    // Fallback: if no sections parsed, treat whole text as one section
    if sections.is_empty() {
        sections.push(Section {
            heading: None,
            body: text.to_string(),
        });
    }

    Ok(Article {
        title,
        summary,
        sections,
    })
}

fn extract_tag(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(text[start..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_article_response() {
        let input = r#"
<article>
<title>测试标题</title>
<summary>这是摘要</summary>
<section>
<heading>第一节</heading>
<body>这是第一段正文。

这是第二段。</body>
</section>
<section>
<body>没有标题的段落。</body>
</section>
</article>"#;

        let article = parse_article_response(input).unwrap();
        assert_eq!(article.title, "测试标题");
        assert_eq!(article.summary, "这是摘要");
        assert_eq!(article.sections.len(), 2);
        assert_eq!(article.sections[0].heading.as_deref(), Some("第一节"));
    }

    #[test]
    fn test_validate_finish_reason_rejects_truncated_output() {
        let err = validate_finish_reason(Some("length"), 4096).unwrap_err();
        assert!(err.to_string().contains("max_tokens=4096"));
    }

    #[test]
    fn test_validate_finish_reason_accepts_complete_output() {
        validate_finish_reason(Some("stop"), 4096).unwrap();
        validate_finish_reason(None, 4096).unwrap();
    }

    #[test]
    fn test_validate_max_tokens_rejects_out_of_range_values() {
        assert!(validate_max_tokens(0).is_err());
        assert!(validate_max_tokens(8193).is_err());
    }

    #[test]
    fn test_validate_max_tokens_accepts_deepseek_limit() {
        assert_eq!(validate_max_tokens(1).unwrap(), 1);
        assert_eq!(validate_max_tokens(8192).unwrap(), 8192);
    }
}
