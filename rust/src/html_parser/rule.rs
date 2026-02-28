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
    /// User-configurable variables (e.g., cookie, token, user_id)
    /// These can be set by users in the app settings
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user_vars: Vec<UserVariable>,
    /// Rule array for complex parsing (chain execution)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<String>,
}

/// User-configurable variable definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserVariable {
    /// Variable name (used in JS as user var)
    pub name: String,
    /// Display label for UI
    pub label: String,
    /// Variable type
    #[serde(default)]
    pub var_type: UserVarType,
    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Placeholder text for input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// Whether this variable is required
    #[serde(default)]
    pub required: bool,
    /// Description/help text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// User variable type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserVarType {
    #[default]
    Text,
    Password,
    Cookie,
    Number,
    Url,
}

/// Search rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRule {
    /// URL pattern, use {keyword} for search term
    pub url: Vec<String>,
    /// Selector for result list
    pub list: Vec<String>,
    pub id: Vec<String>,
    pub title: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_pattern: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Vec<String>>,
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
    pub title: Vec<String>,
    pub list: Vec<String>,
    pub id: Vec<String>,
    #[serde(rename = "itemTitle")]
    pub item_title: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_pattern: Option<Vec<String>>,
}

/// Book detail page rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookDetailRule {
    pub title: Vec<String>,
    pub author: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Chapter list rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterListRule {
    /// If chapters are on a separate page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Vec<String>>,
    pub list: Vec<String>,
    pub id: Vec<String>,
    pub title: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_pattern: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<Vec<String>>,
    /// Next page URL for paginated chapter list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page: Option<Vec<String>>,
    /// Selector to check if there are more pages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_next_page: Option<Vec<String>>,
}

/// Content page rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRule {
    /// For novel: text selector; for comic: image list selector
    pub content: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_url: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_url: Option<Vec<String>>,
    /// Content filter (remove ads, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Vec<String>>,
    /// Next page URL for paginated content (within same chapter)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page: Option<Vec<String>>,
    /// Selector to check if there are more pages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_next_page: Option<Vec<String>>,
}

impl Default for crate::html_parser::dom::ContentType {
    fn default() -> Self {
        Self::Novel
    }
}
