use html_escape::{encode_double_quoted_attribute, encode_text};
use std::collections::HashSet;

use crate::image::EmbeddedImage;
use crate::types::*;

const FONT_FAMILY: &str = "-apple-system, BlinkMacSystemFont, 'Helvetica Neue', 'PingFang SC', 'Hiragino Sans GB', 'Microsoft YaHei', sans-serif";
const MONO_FONT: &str = "'Menlo', 'Monaco', 'Courier New', monospace";
const COLOR_TEXT: &str = "#2b2b2b";
const COLOR_HEADING: &str = "#1a1a1a";
const COLOR_MUTED: &str = "#888888";
const COLOR_ACCENT: &str = "#1e6fff";
const COLOR_LINK: &str = "#576b95";
const COLOR_CODE_BG: &str = "#1e1e1e";
const COLOR_CODE_TEXT: &str = "#d4d4d4";
const COLOR_CODE_BAR_BG: &str = "#2d2d2d";
const COLOR_INLINE_CODE_BG: &str = "#f4f5f7";
const COLOR_INLINE_CODE_TEXT: &str = "#d63384";
const COLOR_QUOTE_BG: &str = "#f7f8fa";
const COLOR_DIVIDER: &str = "#e8e8e8";
const COLOR_BG: &str = "#ffffff";

pub fn render_article(article: &Article, tweet: &TweetData, embedded: &[EmbeddedImage]) -> String {
    let mut html = String::new();
    let all_images: Vec<&str> = tweet
        .posts
        .iter()
        .flat_map(|p| p.images.iter().map(|s| s.as_str()))
        .collect();
    let mut used_images = HashSet::new();

    html.push_str(&format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
</head>
<body style="margin: 0; padding: 40px 20px; background: #ededed; min-height: 100vh;">
<section style="max-width: 640px; margin: 0 auto; background: {COLOR_BG}; border: 1px solid {COLOR_DIVIDER}; padding: 24px 8px; font-family: {FONT_FAMILY}; font-size: 15px; line-height: 1.8; color: {COLOR_TEXT}; word-wrap: break-word; letter-spacing: 0.5px;">
"#,
        title = encode_text(&article.title)
    ));

    html.push_str(&format!(
        r#"<h1 style="font-size: 24px; font-weight: 700; color: {COLOR_HEADING}; text-align: center; margin: 0 0 8px 0; line-height: 1.4;">{}</h1>
<p style="text-align: center; margin: 0 0 4px 0;"><span style="display: inline-block; width: 40px; height: 3px; background: {COLOR_ACCENT}; border-radius: 2px;"></span></p>
"#,
        encode_text(&article.title)
    ));

    if !article.summary.is_empty() {
        html.push_str(&format!(
            r#"<p style="text-align: center; color: {COLOR_MUTED}; font-size: 13px; margin: 0 0 28px 0; line-height: 1.6;">{}</p>
"#,
            encode_text(&article.summary)
        ));
    }

    html.push_str(&render_divider());

    for section in &article.sections {
        if let Some(ref heading) = section.heading {
            html.push_str(&format!(
                r#"<h2 style="font-size: 18px; font-weight: 700; color: {COLOR_HEADING}; margin: 32px 0 16px 0; padding-left: 12px; border-left: 4px solid {COLOR_ACCENT}; line-height: 1.4;">{}</h2>
"#,
                encode_text(heading)
            ));
        }

        html.push_str(&render_body(
            &section.body,
            &all_images,
            embedded,
            &mut used_images,
        ));
    }

    let unused_images: Vec<usize> = all_images
        .iter()
        .enumerate()
        .filter_map(|(idx, _)| (!used_images.contains(&idx)).then_some(idx))
        .collect();

    if !unused_images.is_empty() {
        html.push_str(&render_divider());
        for idx in unused_images {
            html.push_str(&render_image_by_index(
                idx + 1,
                &all_images,
                embedded,
                &mut used_images,
            ));
        }
    }

    html.push_str(&format!(
        r#"<section style="margin: 36px 0 0 0; padding: 20px; background: {COLOR_QUOTE_BG}; border-radius: 8px;">
<p style="font-size: 12px; color: #999; margin: 0 0 4px 0; text-align: center;">原文作者：{author} (@{screen_name})</p>
<p style="font-size: 12px; color: #999; margin: 0 0 4px 0; text-align: center;">来源：X (Twitter)</p>
<p style="font-size: 12px; color: #999; margin: 0; text-align: center;">原文链接：<a href="{url_attr}" style="color: {COLOR_LINK}; text-decoration: none;">{url_text}</a></p>
</section>
"#,
        author = encode_text(&tweet.author.name),
        screen_name = encode_text(&tweet.author.screen_name),
        url_attr = encode_double_quoted_attribute(&tweet.source_url),
        url_text = encode_text(&tweet.source_url),
    ));

    html.push_str(
        r#"</section>
</body>
</html>"#,
    );

    html
}

fn render_divider() -> String {
    format!(
        r#"<section style="margin: 28px auto; text-align: center;">
<span style="display: inline-block; width: 6px; height: 6px; background: {COLOR_ACCENT}; border-radius: 50%; margin: 0 6px;"></span>
<span style="display: inline-block; width: 6px; height: 6px; background: {COLOR_ACCENT}; border-radius: 50%; opacity: 0.5; margin: 0 6px;"></span>
<span style="display: inline-block; width: 6px; height: 6px; background: {COLOR_ACCENT}; border-radius: 50%; opacity: 0.25; margin: 0 6px;"></span>
</section>
"#
    )
}

#[derive(PartialEq)]
enum BlockMode {
    Plain,
    Code { lang: Option<String> },
    Quote,
    UnorderedList,
    OrderedList,
}

fn render_body(
    body: &str,
    all_images: &[&str],
    embedded: &[EmbeddedImage],
    used_images: &mut HashSet<usize>,
) -> String {
    let mut result = String::new();
    let mut mode = BlockMode::Plain;
    let mut buf: Vec<String> = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") {
            match mode {
                BlockMode::Code { .. } => {
                    let lang = if let BlockMode::Code { ref lang } = mode {
                        lang.clone()
                    } else {
                        None
                    };
                    result.push_str(&render_code_block(lang.as_deref(), &buf.join("\n")));
                    buf.clear();
                    mode = BlockMode::Plain;
                    continue;
                }
                _ => {
                    flush_block(
                        &mut result,
                        &mode,
                        &mut buf,
                        all_images,
                        embedded,
                        used_images,
                    );
                    let lang_str = trimmed[3..].trim();
                    let lang = if lang_str.is_empty() {
                        None
                    } else {
                        Some(lang_str.to_string())
                    };
                    mode = BlockMode::Code { lang };
                    continue;
                }
            }
        }

        if let BlockMode::Code { .. } = mode {
            buf.push(line.to_string());
            continue;
        }

        if let Some(image_idx) = parse_image_placeholder(trimmed) {
            flush_block(
                &mut result,
                &mode,
                &mut buf,
                all_images,
                embedded,
                used_images,
            );
            mode = BlockMode::Plain;
            result.push_str(&render_image_by_index(
                image_idx,
                all_images,
                embedded,
                used_images,
            ));
            continue;
        }

        let line_mode = detect_line_type(trimmed);

        if line_mode != mode {
            flush_block(
                &mut result,
                &mode,
                &mut buf,
                all_images,
                embedded,
                used_images,
            );
            mode = line_mode;
        }

        match mode {
            BlockMode::Quote => {
                let content = trimmed
                    .strip_prefix("> ")
                    .unwrap_or(trimmed.strip_prefix(">").unwrap_or(trimmed));
                buf.push(content.to_string());
            }
            BlockMode::UnorderedList => {
                let content = trimmed.strip_prefix("- ").unwrap_or(&trimmed[2..]);
                buf.push(content.to_string());
            }
            BlockMode::OrderedList => {
                if let Some(pos) = trimmed.find(". ") {
                    buf.push(trimmed[pos + 2..].to_string());
                } else {
                    buf.push(trimmed.to_string());
                }
            }
            BlockMode::Plain => buf.push(line.to_string()),
            BlockMode::Code { .. } => unreachable!(),
        }
    }

    if let BlockMode::Code { .. } = mode {
        let mut plain_buf: Vec<String> = vec!["```".to_string()];
        plain_buf.append(&mut buf);
        flush_block(
            &mut result,
            &BlockMode::Plain,
            &mut plain_buf,
            all_images,
            embedded,
            used_images,
        );
    } else {
        flush_block(
            &mut result,
            &mode,
            &mut buf,
            all_images,
            embedded,
            used_images,
        );
    }

    result
}

fn detect_line_type(trimmed: &str) -> BlockMode {
    if trimmed.starts_with('>') {
        BlockMode::Quote
    } else if trimmed.starts_with("- ") {
        BlockMode::UnorderedList
    } else if is_ordered_list_line(trimmed) {
        BlockMode::OrderedList
    } else {
        BlockMode::Plain
    }
}

fn is_ordered_list_line(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    i > 0 && i + 1 < bytes.len() && bytes[i] == b'.' && bytes[i + 1] == b' '
}

fn flush_block(
    result: &mut String,
    mode: &BlockMode,
    buf: &mut Vec<String>,
    all_images: &[&str],
    embedded: &[EmbeddedImage],
    used_images: &mut HashSet<usize>,
) {
    if buf.is_empty() {
        return;
    }

    match mode {
        BlockMode::Plain => {
            let text = buf.join("\n");
            result.push_str(&render_paragraphs(&text, all_images, embedded, used_images));
        }
        BlockMode::Quote => {
            let text = buf.join("\n");
            result.push_str(&render_blockquote(&text));
        }
        BlockMode::UnorderedList => {
            let items: Vec<&str> = buf.iter().map(|s| s.as_str()).collect();
            result.push_str(&render_list_items(&items, false));
        }
        BlockMode::OrderedList => {
            let items: Vec<&str> = buf.iter().map(|s| s.as_str()).collect();
            result.push_str(&render_list_items(&items, true));
        }
        BlockMode::Code { .. } => {}
    }

    buf.clear();
}

fn render_paragraphs(
    text: &str,
    all_images: &[&str],
    embedded: &[EmbeddedImage],
    used_images: &mut HashSet<usize>,
) -> String {
    let mut result = String::new();

    for para in text.split("\n\n") {
        let trimmed = para.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(image_idx) = parse_image_placeholder(trimmed) {
            result.push_str(&render_image_by_index(
                image_idx,
                all_images,
                embedded,
                used_images,
            ));
            continue;
        }

        if trimmed.starts_with("## ") {
            result.push_str(&format!(
                r#"<h2 style="font-size: 18px; font-weight: 700; color: {COLOR_HEADING}; margin: 32px 0 16px 0; padding-left: 12px; border-left: 4px solid {COLOR_ACCENT}; line-height: 1.4;">{}</h2>
"#,
                encode_text(&trimmed[3..])
            ));
        } else if trimmed.starts_with("# ") {
            result.push_str(&format!(
                r#"<h2 style="font-size: 18px; font-weight: 700; color: {COLOR_HEADING}; margin: 32px 0 16px 0; padding-left: 12px; border-left: 4px solid {COLOR_ACCENT}; line-height: 1.4;">{}</h2>
"#,
                encode_text(&trimmed[2..])
            ));
        } else {
            result.push_str(&render_paragraph(trimmed));
            result.push('\n');
        }
    }

    result
}

fn parse_image_placeholder(text: &str) -> Option<usize> {
    let inner = text
        .trim()
        .strip_prefix("[[IMAGE:")?
        .strip_suffix("]]")?;
    inner.parse::<usize>().ok().filter(|idx| *idx > 0)
}

fn render_image_by_index(
    index: usize,
    all_images: &[&str],
    embedded: &[EmbeddedImage],
    used_images: &mut HashSet<usize>,
) -> String {
    let Some(zero_based) = index.checked_sub(1) else {
        return String::new();
    };
    let Some(img_url) = all_images.get(zero_based).copied() else {
        return String::new();
    };

    used_images.insert(zero_based);

    if let Some(ei) = embedded.iter().find(|e| e.original_url == img_url) {
        format!(
            r#"<section style="text-align: center; margin: 24px 0;"><img src="data:{};base64,{}" style="max-width: 100%; border-radius: 8px; box-shadow: 0 2px 12px rgba(0,0,0,0.08);" /></section>
"#,
            ei.mime_type, ei.base64_data
        )
    } else {
        format!(
            r#"<!-- 图片下载失败，请手动上传此图片: {} -->
<section style="text-align: center; margin: 24px 0;"><img src="{}" style="max-width: 100%; border-radius: 8px; box-shadow: 0 2px 12px rgba(0,0,0,0.08);" /></section>
"#,
            encode_text(img_url),
            encode_text(img_url)
        )
    }
}

fn render_code_block(lang: Option<&str>, code: &str) -> String {
    let mut html = format!(
        r#"<section style="margin: 20px 0; background: {COLOR_CODE_BG}; border-radius: 8px; overflow: hidden;">"#
    );

    if let Some(lang) = lang {
        if !lang.is_empty() {
            html.push_str(&format!(
                r#"
<section style="padding: 8px 16px; background: {COLOR_CODE_BAR_BG}; border-bottom: 1px solid #3d3d3d;"><span style="font-size: 12px; color: #888; font-family: {MONO_FONT};">{}</span></section>"#,
                encode_text(lang)
            ));
        }
    }

    html.push_str(&format!(
        r#"
<pre style="margin: 0; padding: 16px; font-family: {MONO_FONT}; font-size: 13px; line-height: 1.6; color: {COLOR_CODE_TEXT}; white-space: pre-wrap; word-wrap: break-word; overflow-x: auto;">{}</pre>
</section>
"#,
        encode_text(code)
    ));

    html
}

fn render_blockquote(text: &str) -> String {
    let inner = render_inline(text);
    format!(
        r#"<blockquote style="margin: 20px 0; padding: 16px 20px; background: {COLOR_QUOTE_BG}; border-left: 4px solid {COLOR_ACCENT}; border-radius: 0 8px 8px 0; color: #555; font-size: 14px; line-height: 1.75;">
<p style="margin: 0;">{inner}</p>
</blockquote>
"#
    )
}

fn render_list_items(items: &[&str], ordered: bool) -> String {
    let mut html = String::from(r#"<section style="margin: 0 0 20px 0; padding-left: 0;">"#);
    html.push('\n');

    for (i, item) in items.iter().enumerate() {
        let marker = if ordered {
            format!(
                r#"<span style="color: {COLOR_ACCENT}; font-weight: 600; margin-right: 8px;">{}.</span>"#,
                i + 1
            )
        } else {
            format!(r#"<span style="color: {COLOR_ACCENT}; font-weight: bold; margin-right: 8px;">•</span>"#)
        };
        let content = render_inline(item);
        html.push_str(&format!(
            r#"<section style="margin: 0 0 8px 0; padding-left: 20px;">{marker}<span>{content}</span></section>
"#
        ));
    }

    html.push_str("</section>\n");
    html
}

fn render_paragraph(text: &str) -> String {
    let inner = render_inline(text);
    format!(
        r#"<p style="margin: 0 0 20px 0; text-align: justify; color: {COLOR_TEXT};">{inner}</p>"#
    )
}

fn render_inline(text: &str) -> String {
    let mut result = String::new();
    let mut i: usize = 0;
    let mut plain_start = 0;

    while i < text.len() {
        let current = &text[i..];

        if current.starts_with('`') {
            if let Some(end_rel) = current[1..].find('`') {
                let end = i + 1 + end_rel;
                result.push_str(encode_text(&text[plain_start..i]).as_ref());
                let code_content = &text[i + 1..end];
                result.push_str(&format!(
                    r#"<code style="background: {COLOR_INLINE_CODE_BG}; color: {COLOR_INLINE_CODE_TEXT}; padding: 2px 6px; border-radius: 4px; font-family: {MONO_FONT}; font-size: 13px;">{}</code>"#,
                    encode_text(code_content)
                ));
                plain_start = end + 1;
                i = end + 1;
            } else {
                i += '`'.len_utf8();
            }
        } else if current.starts_with("**") {
            if let Some(end_rel) = current[2..].find("**") {
                let end = i + 2 + end_rel;
                result.push_str(encode_text(&text[plain_start..i]).as_ref());
                let bold_content = &text[i + 2..end];
                result.push_str(&format!(
                    r#"<strong style="color: {COLOR_HEADING}; font-weight: 600;">{}</strong>"#,
                    encode_text(bold_content)
                ));
                plain_start = end + 2;
                i = end + 2;
            } else {
                i += '*'.len_utf8();
            }
        } else if current.starts_with('[') {
            if let Some(label_end_rel) = current[1..].find("](") {
                let label_end = i + 1 + label_end_rel;
                if let Some(url_end_rel) = text[label_end + 2..].find(')') {
                    let url_end = label_end + 2 + url_end_rel;
                    result.push_str(encode_text(&text[plain_start..i]).as_ref());
                    let label = &text[i + 1..label_end];
                    let url = &text[label_end + 2..url_end];
                    result.push_str(&format!(
                        r#"<a href="{}" style="color: {COLOR_LINK}; text-decoration: none;">{}</a>"#,
                        encode_double_quoted_attribute(url),
                        encode_text(label)
                    ));
                    plain_start = url_end + 1;
                    i = url_end + 1;
                } else {
                    i += '['.len_utf8();
                }
            } else {
                i += '['.len_utf8();
            }
        } else {
            i += current.chars().next().map(char::len_utf8).unwrap_or(1);
        }
    }

    if plain_start < text.len() {
        result.push_str(encode_text(&text[plain_start..]).as_ref());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_body_for_test(body: &str) -> String {
        let mut used_images = HashSet::new();
        render_body(body, &[], &[], &mut used_images)
    }

    #[test]
    fn test_render_code_block() {
        let result = render_code_block(None, "let x = 1;");
        assert!(result.contains("background: #1e1e1e"));
        assert!(result.contains("<pre"));
        assert!(result.contains("<section"));
        assert!(result.contains("let x = 1;"));
        assert!(!result.contains("#2d2d2d"));
    }

    #[test]
    fn test_render_code_block_with_lang() {
        let result = render_code_block(Some("rust"), "fn main() {}");
        assert!(result.contains("rust"));
        assert!(result.contains("#2d2d2d"));
        assert!(result.contains("fn main() {}"));
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
        let result = render_body_for_test(body);
        assert!(result.contains("<p"));
        assert!(result.contains("<pre"));
        assert!(result.contains("<code"));
        assert!(result.contains("rust"));
    }

    #[test]
    fn test_unmatched_backtick() {
        let text = "这是一个未闭合的 ` 反引号";
        let result = render_paragraph(text);
        assert!(result.contains("` 反引号"));
    }

    #[test]
    fn test_render_bold() {
        let text = "这是**加粗文本**示例";
        let result = render_paragraph(text);
        assert!(result.contains("<strong"));
        assert!(result.contains("加粗文本"));
        assert!(result.contains("</strong>"));
    }

    #[test]
    fn test_render_bold_and_code_mixed() {
        let text = "使用 `command` 来实现**重要功能**";
        let result = render_paragraph(text);
        assert!(result.contains("<code"));
        assert!(result.contains("command"));
        assert!(result.contains("<strong"));
        assert!(result.contains("重要功能"));
    }

    #[test]
    fn test_render_blockquote() {
        let body = "> 这是一段引用\n> 第二行引用";
        let result = render_body_for_test(body);
        assert!(result.contains("<blockquote"));
        assert!(result.contains("这是一段引用"));
        assert!(result.contains("第二行引用"));
        assert!(result.contains(COLOR_ACCENT));
    }

    #[test]
    fn test_render_unordered_list() {
        let body = "- 第一项\n- 第二项\n- 第三项";
        let result = render_body_for_test(body);
        assert!(result.contains("•"));
        assert!(result.contains("第一项"));
        assert!(result.contains("第三项"));
    }

    #[test]
    fn test_render_ordered_list() {
        let body = "1. 第一步\n2. 第二步\n3. 第三步";
        let result = render_body_for_test(body);
        assert!(result.contains("1."));
        assert!(result.contains("2."));
        assert!(result.contains("第一步"));
        assert!(result.contains("第三步"));
    }

    #[test]
    fn test_render_markdown_link() {
        let text = "参考[官方文档](https://example.com/docs)继续阅读";
        let result = render_paragraph(text);
        assert!(result.contains("<a href=\"https://example.com/docs\""));
        assert!(result.contains("官方文档"));
        assert!(result.contains(COLOR_LINK));
    }

    #[test]
    fn test_render_divider() {
        let result = render_divider();
        assert!(result.contains(COLOR_ACCENT));
        assert!(result.contains("border-radius: 50%"));
        assert!(result.contains("opacity: 0.5"));
    }

    #[test]
    fn test_render_image_placeholder_inline() {
        let mut used_images = HashSet::new();
        let result = render_body(
            "第一段\n\n[[IMAGE:1]]\n\n第二段",
            &["https://example.com/a.png"],
            &[],
            &mut used_images,
        );
        assert!(result.contains("https://example.com/a.png"));
        assert!(result.contains("第一段"));
        assert!(result.contains("第二段"));
        assert!(used_images.contains(&0));
    }
}
