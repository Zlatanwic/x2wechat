use std::fmt;

#[derive(Debug)]
pub enum Tweet2WxError {
    InvalidUrl(String),
    FetchFailed(String),
    ApiError(String),
    ParseError(String),
}

impl fmt::Display for Tweet2WxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(msg) => write!(f, "Invalid tweet URL: {msg}"),
            Self::FetchFailed(msg) => write!(f, "Failed to fetch tweet: {msg}"),
            Self::ApiError(msg) => write!(f, "API error: {msg}"),
            Self::ParseError(msg) => write!(f, "Parse error: {msg}"),
        }
    }
}

impl std::error::Error for Tweet2WxError {}
