use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "tweet2wx", about = "Convert X/Twitter posts to WeChat articles")]
pub struct Args {
    /// Tweet URL (e.g. https://x.com/user/status/123456789)
    pub url: String,

    /// Output HTML file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Article style: informative, casual, technical
    #[arg(short, long, default_value = "informative")]
    pub style: String,

    /// Override LLM model name
    #[arg(long, default_value = "deepseek-chat")]
    pub model: String,
}

impl Args {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}