//! Rule definitions for content sources

use serde::{Deserialize, Serialize};

/// Content source with parsing rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub content_type: crate::html_parser::dom::ContentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<SearchRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explore: Option<ExploreRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book_detail: Option<BookDetailRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_list: Option<ChapterListRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ContentRule>,
    /// Custom headers for requests
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub headers: std::collections::HashMap<String, String>,
}

/// Search rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRule {
    /// URL pattern, use {keyword} for search term
    pub url: String,
    /// Selector for result list
    pub list: String,
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Explore/Discovery rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExploreRule {
    pub url: String,
    /// Multiple sections on explore page
    #[serde(default)]
    pub sections: Vec<ExploreSectionRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExploreSectionRule {
    pub title: String,
    pub list: String,
    pub id: String,
    #[serde(rename = "itemTitle")]
    pub item_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_pattern: Option<String>,
}

/// Book detail page rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookDetailRule {
    pub title: String,
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
}

/// Chapter list rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterListRule {
    /// If chapters are on a separate page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub list: String,
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// Content page rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRule {
    pub title: String,
    /// For novel: text selector; for comic: image list selector
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_url: Option<String>,
    /// Content filter (remove ads, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

impl Default for crate::html_parser::dom::ContentType {
    fn default() -> Self {
        Self::Novel
    }
}
