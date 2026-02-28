use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RequestMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl Default for RequestMethod {
    fn default() -> Self {
        Self::Get
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestConfig {
    #[serde(default)]
    pub method: RequestMethod,
    
    pub url: String,
    
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
    
    #[serde(rename = "contentType", skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

impl RequestConfig {
    pub fn new(url: String) -> Self {
        Self {
            method: RequestMethod::Get,
            url,
            headers: HashMap::new(),
            content_type: None,
            body: None,
        }
    }
}
