mod cli;
mod config;
mod error;
mod fetcher;
mod html;
mod image;
mod llm;
mod types;

use anyhow::Result;
use cli::Args;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse_args();

    // 1. Load config (API key from env / config file)
    let config = config::Config::load()?;

    // 2. Fetch tweet content via FxTwitter API
    println!("🔍 Fetching tweet...");
    let tweet = fetcher::fetch_tweet(&args.url).await?;
    println!(
        "✅ Fetched: @{} ({} posts in thread)",
        tweet.author.screen_name,
        tweet.posts.len()
    );

    // 3. Translate & rewrite via Claude API
    println!("🤖 Translating & rewriting...");
    let article = llm::translate_and_rewrite(&config, &tweet, &args).await?;
    println!("✅ Article generated: {}", article.title);

    // 4. Download and embed images as base64
    let all_image_urls: Vec<String> = tweet.posts.iter().flat_map(|p| p.images.clone()).collect();
    let embedded_images = if all_image_urls.is_empty() {
        Vec::new()
    } else {
        println!("🖼️  Downloading {} image(s)...", all_image_urls.len());
        let images = image::download_and_embed(&all_image_urls).await;
        println!("✅ Embedded {} image(s)", images.len());
        images
    };

    // 5. Render to WeChat-compatible HTML
    println!("📝 Rendering HTML...");
    let html_content = html::render_article(&article, &tweet, &embedded_images);

    // 6. Write output file
    let output_path = args.output.unwrap_or_else(|| "output.html".into());
    std::fs::write(&output_path, &html_content)?;
    println!("🎉 Done! Output: {}", output_path);
    println!("   Open in browser → Select All → Copy → Paste to WeChat editor");

    Ok(())
}
