use thiserror::Error;
use crate::cookie::CookieError;

pub type Result<T> = std::result::Result<T, HttpClientError>;

#[derive(Error, Debug)]
pub enum HttpClientError {
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    
    #[error("Cookie error: {0}")]
    Cookie(#[from] CookieError),
    
    #[error("Invalid header: {0}")]
    InvalidHeader(String),
    
    #[error("Request failed with status: {0}")]
    RequestFailed(u16),
}
