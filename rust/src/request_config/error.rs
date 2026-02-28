use thiserror::Error;

pub type Result<T> = std::result::Result<T, RequestConfigError>;

#[derive(Error, Debug)]
pub enum RequestConfigError {
    #[error("Template error: {0}")]
    Template(String),
    
    #[error("Variable not found: {0}")]
    VariableNotFound(String),
    
    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
}
