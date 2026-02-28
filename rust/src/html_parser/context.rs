//! Parse context for rule engine
//!
//! Provides shared state across parsing steps (book info → chapter list → content).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parse context with shared variables
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParseContext {
    /// Current book info (populated after parsing book detail)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book: Option<BookContext>,

    /// Source configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceContext>,

    /// User-defined variables (e.g., cookie, token)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub user_vars: HashMap<String, String>,

    /// Shared variables across parsing steps
    /// JS can read/write via `shared.get(key)` / `shared.set(key, value)`
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub shared: HashMap<String, String>,

    /// Current page info (for multi-page content)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<PageContext>,

    /// Parse mode: which data to extract from current HTML
    #[serde(default)]
    pub mode: ParseMode,
}

/// Book context available in JS as `book` object
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BookContext {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Extra fields
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, String>,
}

/// Source context available in JS as `source` object
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceContext {
    pub id: String,
    pub name: String,
    pub base_url: String,
    /// Headers including user-set cookies
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
}

/// Page context for multi-page content
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageContext {
    /// Current page index (0-based)
    pub index: usize,
    /// Total pages (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
    /// Current page URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Next page URL (if exists)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_url: Option<String>,
    /// Previous page URL (if exists)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_url: Option<String>,
}

/// Parse mode - what to extract from HTML
/// Supports scenarios where book info, chapters, and content are on same page
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseMode {
    /// Parse book detail only
    #[default]
    BookDetail,
    /// Parse chapter list only
    ChapterList,
    /// Parse content only
    Content,
    /// Parse book detail + chapter list (common for novels)
    BookAndChapters,
    /// Parse all: book detail + chapters + content (e.g., anime single page)
    All,
}

impl ParseContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create context with source info
    pub fn with_source(source_id: &str, source_name: &str, base_url: &str) -> Self {
        Self {
            source: Some(SourceContext {
                id: source_id.to_string(),
                name: source_name.to_string(),
                base_url: base_url.to_string(),
                headers: HashMap::new(),
            }),
            ..Default::default()
        }
    }

    /// Set book context
    pub fn set_book(&mut self, book: BookContext) {
        self.book = Some(book);
    }

    /// Set user variable (e.g., cookie)
    pub fn set_user_var(&mut self, key: &str, value: &str) {
        self.user_vars.insert(key.to_string(), value.to_string());
    }

    /// Get user variable
    pub fn get_user_var(&self, key: &str) -> Option<&String> {
        self.user_vars.get(key)
    }

    /// Set shared variable (accessible across parse steps)
    pub fn set_shared(&mut self, key: &str, value: &str) {
        self.shared.insert(key.to_string(), value.to_string());
    }

    /// Get shared variable
    pub fn get_shared(&self, key: &str) -> Option<&String> {
        self.shared.get(key)
    }

    /// Set page context
    pub fn set_page(&mut self, page: PageContext) {
        self.page = Some(page);
    }

    /// Convert to HashMap for JS variable injection
    pub fn to_vars(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();

        // Inject book as JSON
        if let Some(book) = &self.book {
            if let Ok(json) = serde_json::to_string(book) {
                vars.insert("__book_json".to_string(), json);
            }
        }

        // Inject source as JSON
        if let Some(source) = &self.source {
            if let Ok(json) = serde_json::to_string(source) {
                vars.insert("__source_json".to_string(), json);
            }
        }

        // Inject page as JSON
        if let Some(page) = &self.page {
            if let Ok(json) = serde_json::to_string(page) {
                vars.insert("__page_json".to_string(), json);
            }
        }

        // Inject shared vars as JSON
        if !self.shared.is_empty() {
            if let Ok(json) = serde_json::to_string(&self.shared) {
                vars.insert("__shared_json".to_string(), json);
            }
        }

        // Inject user vars directly
        for (k, v) in &self.user_vars {
            vars.insert(k.clone(), v.clone());
        }

        // Inject parse mode
        vars.insert(
            "__parse_mode".to_string(),
            match self.mode {
                ParseMode::BookDetail => "book_detail".to_string(),
                ParseMode::ChapterList => "chapter_list".to_string(),
                ParseMode::Content => "content".to_string(),
                ParseMode::BookAndChapters => "book_and_chapters".to_string(),
                ParseMode::All => "all".to_string(),
            },
        );

        vars
    }
}

/// Result from parsing that may request WebView rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ParseResult {
    /// Successfully parsed data
    Success {
        data: serde_json::Value,
        /// Updated shared variables from JS
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        shared_updates: HashMap<String, String>,
    },
    /// Request WebView to render a URL and return HTML
    WebViewRequest {
        url: String,
        /// Optional JS to execute after page load
        #[serde(skip_serializing_if = "Option::is_none")]
        js: Option<String>,
        /// Wait condition (CSS selector or timeout in ms)
        #[serde(skip_serializing_if = "Option::is_none")]
        wait_for: Option<String>,
    },
    /// Multi-page content with next page URL
    HasNextPage {
        data: serde_json::Value,
        next_url: String,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        shared_updates: HashMap<String, String>,
    },
    /// Error during parsing
    Error { message: String },
}
