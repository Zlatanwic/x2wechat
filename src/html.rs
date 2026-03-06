use html_escape::encode_text;

use crate::image::EmbeddedImage;
use crate::types::*;

const CODE_PRE_STYLE: &str = "background: #1e1e1e; border-radius: 6px; padding: 16px; margin: 16px 0; overflow-x: auto; font-family: 'Menlo', 'Monaco', 'Courier New', monospace; font-size: 13px; line-height: 1.6; color: #d4d4d4; white-space: pre-wrap; word-wrap: break-word;";
const INLINE_CODE_STYLE: &str = "background: #f0f0f0; color: #e01e5a; padding: 2px 6px; border-radius: 3px; font-family: 'Menlo', 'Monaco', 'Courier New', monospace; font-size: 14px;";
const PARAGRAPH_STYLE: &str = "margin: 0 0 16px 0; text-align: justify;";

/// Render an article to WeChat-compatible inline-styled HTML
pub fn render_article(article: &Article, tweet: &TweetData, embedded: &[EmbeddedImage]) -> String {
    let mut html = String::new();

    // Outer container
    html.push_str(&format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<style>
  /* This style block is for local preview only.
     WeChat editor will strip it — all visible styles are inline. */
  body {{ margin: 0; padding: 20px; background: #f5f5f5; }}
</style>
</head>
<body>
<section id="wx-article" style="max-width: 640px; margin: 0 auto; background: #fff; padding: 24px 20px; font-family: -apple-system, BlinkMacSystemFont, 'Helvetica Neue', 'PingFang SC', 'Microsoft YaHei', sans-serif; font-size: 16px; line-height: 1.8; color: #333; word-wrap: break-word;">
"#,
        title = encode_text(&article.title)
    ));

    // Title
    html.push_str(&format!(
        r#"<h1 style="font-size: 22px; font-weight: bold; color: #1a1a1a; text-align: center; margin-bottom: 8px; line-height: 1.4;">{}</h1>
"#,
        encode_text(&article.title)
    ));

    // Summary / subtitle
    if !article.summary.is_empty() {
        html.push_str(&format!(
            r#"<p style="text-align: center; color: #888; font-size: 14px; margin-bottom: 24px;">{}</p>
"#,
            encode_text(&article.summary)
        ));
    }

    // Divider
    html.push_str(
        r#"<hr style="border: none; border-top: 1px solid #eee; margin: 20px 0;" />
"#,
    );

    // Sections
    for section in &article.sections {
        if let Some(ref heading) = section.heading {
            html.push_str(&format!(
                r#"<h2 style="font-size: 18px; font-weight: bold; color: #1a1a1a; margin: 24px 0 12px 0; padding-left: 10px; border-left: 4px solid #07c160;">{}</h2>
"#,
                encode_text(heading)
            ));
        }

        html.push_str(&render_body(&section.body));
    }

    // Images from the original tweet
    let all_images: Vec<&str> = tweet
        .posts
        .iter()
        .flat_map(|p| p.images.iter().map(|s| s.as_str()))
        .collect();

    if !all_images.is_empty() {
        html.push_str(
            r#"<hr style="border: none; border-top: 1px solid #eee; margin: 20px 0;" />
"#,
        );
        for img_url in &all_images {
            if let Some(ei) = embedded.iter().find(|e| e.original_url == *img_url) {
                html.push_str(&format!(
                    r#"<p style="text-align: center; margin: 12px 0;"><img src="data:{};base64,{}" style="max-width: 100%; border-radius: 4px;" /></p>
"#,
                    ei.mime_type, ei.base64_data
                ));
            } else {
                html.push_str(&format!(
                    r#"<!-- ⚠️ 图片下载失败，请手动上传此图片: {} -->
<p style="text-align: center; margin: 12px 0;"><img src="{}" style="max-width: 100%; border-radius: 4px;" /></p>
"#,
                    encode_text(img_url),
                    encode_text(img_url)
                ));
            }
        }
    }

    // Footer: source attribution
    html.push_str(&format!(
        r#"
<hr style="border: none; border-top: 1px solid #eee; margin: 24px 0 16px 0;" />
<p style="font-size: 12px; color: #999; text-align: center; margin: 0;">
  原文作者：{author} (@{screen_name})<br />
  来源：X (Twitter)<br />
  原文链接：{url}
</p>
"#,
        author = encode_text(&tweet.author.name),
        screen_name = encode_text(&tweet.author.screen_name),
        url = encode_text(&tweet.source_url),
    ));

    // Close container
    html.push_str(
        r#"</section>
</body>
</html>"#,
    );

    html
}

/// Render section body with support for fenced code blocks and inline code.
fn render_body(body: &str) -> String {
    let mut result = String::new();
    let mut text_buf = String::new();
    let mut in_code_block = false;
    let mut code_buf = String::new();

    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            if in_code_block {
                // End of code block
                result.push_str(&render_paragraphs(&text_buf));
                text_buf.clear();
                result.push_str(&render_code_block(&code_buf));
                code_buf.clear();
                in_code_block = false;
            } else {
                // Start of code block — flush pending text later when block ends
                in_code_block = true;
            }
        } else if in_code_block {
            if !code_buf.is_empty() {
                code_buf.push('\n');
            }
            code_buf.push_str(line);
        } else {
            if !text_buf.is_empty() {
                text_buf.push('\n');
            }
            text_buf.push_str(line);
        }
    }

    // If code block was never closed, treat the opening ``` and content as plain text
    if in_code_block {
        if !text_buf.is_empty() {
            text_buf.push('\n');
        }
        text_buf.push_str("```\n");
        text_buf.push_str(&code_buf);
    }

    result.push_str(&render_paragraphs(&text_buf));
    result
}

fn render_paragraphs(text: &str) -> String {
    let mut result = String::new();
    for para in text.split("\n\n") {
        let trimmed = para.trim();
        if trimmed.is_empty() {
            continue;
        }
        result.push_str(&render_paragraph(trimmed));
        result.push('\n');
    }
    result
}

fn render_code_block(code: &str) -> String {
    format!(
        "<pre style=\"{CODE_PRE_STYLE}\">{}</pre>\n",
        encode_text(code)
    )
}

fn render_paragraph(text: &str) -> String {
    let inner = render_inline_code(text);
    format!("<p style=\"{PARAGRAPH_STYLE}\">{inner}</p>")
}

/// Process inline code (`...`) in text. Returns HTML with <code> tags.
fn render_inline_code(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.char_indices().peekable();
    let mut plain_start = 0;

    while let Some(&(i, ch)) = chars.peek() {
        if ch == '`' {
            // Look for closing backtick
            chars.next(); // consume opening `
            let code_start = i + 1;
            let mut found_end = false;
            while let Some(&(j, c)) = chars.peek() {
                chars.next();
                if c == '`' {
                    // Emit plain text before this inline code
                    result.push_str(encode_text(&text[plain_start..i]).as_ref());
                    // Emit code
                    let code_content = &text[code_start..j];
                    result.push_str(&format!(
                        "<code style=\"{INLINE_CODE_STYLE}\">{}</code>",
                        encode_text(code_content)
                    ));
                    plain_start = j + 1;
                    found_end = true;
                    break;
                }
            }
            if !found_end {
                // No closing backtick — treat as plain text, continue
            }
        } else {
            chars.next();
        }
    }

    // Remaining plain text
    if plain_start < text.len() {
        result.push_str(encode_text(&text[plain_start..]).as_ref());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_code_block() {
        let code = "let x = 1;\nprintln!(\"{}\", x);";
        let result = render_code_block(code);
        assert!(result.contains("background: #1e1e1e"));
        assert!(result.contains("<pre"));
        assert!(!result.contains("<section")); // no <section> wrapper
        assert!(result.contains("let x = 1;"));
    }

    #[test]
    fn test_render_paragraph_with_inline_code() {
        let text = "运行 `cargo build` 然后执行 `./app`";
        let result = render_paragraph(text);
        assert!(result.contains("<code"));
        assert!(result.contains("cargo build"));
        assert!(result.contains("./app"));
        assert!(result.contains("运行"));
    }

    #[test]
    fn test_render_body_mixed() {
        let body = "这是普通段落。\n\n```rust\nfn main() {}\n```\n\n这是 `行内代码` 示例。";
        let result = render_body(body);
        assert!(result.contains("<p"));
        assert!(result.contains("<pre"));
        assert!(result.contains("<code"));
    }

    #[test]
    fn test_unmatched_backtick() {
        let text = "这是一个未闭合的 ` 反引号";
        let result = render_paragraph(text);
        assert!(result.contains("` 反引号"));
    }
}