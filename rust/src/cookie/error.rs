use thiserror::Error;

pub type Result<T> = std::result::Result<T, CookieError>;

#[derive(Error, Debug)]
pub enum CookieError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    
    #[error("Cookie not found: {0}")]
    NotFound(String),
    
    #[error("Invalid cookie: {0}")]
    InvalidCookie(String),
}
