//! Rule parser - evaluates selectors against HTML/JSON
//!
//! Selector format:
//! - `@css:.selector` - CSS selector, get text
//! - `@css:.selector@attr` - CSS selector, get attribute
//! - `@js:script` - JavaScript expression
//! - `@re:pattern` - Regex on raw HTML
//! - `@re:pattern##replacement` - Regex replace
//! - `@json:$.path` - JSON Path selector (for JSON content)
//! - Plain text - literal value

use regex::Regex;

use crate::js_engine::JsRuntime;

/// Rule evaluation error
#[derive(Debug)]
pub enum RuleError {
    InvalidSelector(String),
    JsError(String),
    RegexError(String),
    JsonPathError(String),
    WebViewNotSupported,
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSelector(s) => write!(f, "Invalid selector: {}", s),
            Self::JsError(s) => write!(f, "JS error: {}", s),
            Self::RegexError(s) => write!(f, "Regex error: {}", s),
            Self::JsonPathError(s) => write!(f, "JSONPath error: {}", s),
            Self::WebViewNotSupported => write!(f, "WebView not supported"),
        }
    }
}

impl std::error::Error for RuleError {}

/// Rule parser with cached JS runtime
pub struct RuleParser {
    runtime: JsRuntime,
    html: String,
}

impl RuleParser {
    pub fn new(html: &str) -> Self {
        Self {
            runtime: JsRuntime::new(),
            html: html.to_string(),
        }
    }

    /// Evaluate a single selector rule
    pub fn eval(&mut self, rule: &str) -> Result<String, RuleError> {
        let rule = rule.trim();

        if rule.starts_with("@css:") {
            self.eval_css(&rule[5..])
        } else if rule.starts_with("@js:") {
            self.eval_js(&rule[4..])
        } else if rule.starts_with("@re:") {
            self.eval_regex(&rule[4..])
        } else if rule.starts_with("@json:") {
            self.eval_json(&rule[6..])
        } else if rule.starts_with("@webview:") {
            Err(RuleError::WebViewNotSupported)
        } else if rule.starts_with(".") || rule.starts_with("#") || rule.contains(" ") {
            self.eval_css(rule)
        } else {
            Ok(rule.to_string())
        }
    }

    /// Evaluate an array of rules (chain execution)
    /// Each rule's result is passed as input to the next rule
    pub fn eval_rules(&mut self, rules: &[String]) -> Result<String, RuleError> {
        let mut result = self.html.clone();

        for rule in rules {
            let rule = rule.trim();

            // Create a temporary parser with the current result as input
            let mut temp_parser = RuleParser::new(&result);
            result = temp_parser.eval(rule)?;
        }

        Ok(result)
    }

    /// Evaluate an optional array of rules
    pub fn eval_opt_rules(
        &mut self,
        rules: &Option<Vec<String>>,
    ) -> Result<Option<String>, RuleError> {
        match rules {
            Some(rules) if !rules.is_empty() => {
                let result = self.eval_rules(rules)?;
                if result.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(result))
                }
            }
            _ => Ok(None),
        }
    }

    fn eval_css(&mut self, selector: &str) -> Result<String, RuleError> {
        let (sel, attr) = if let Some(pos) = selector.rfind('@') {
            (&selector[..pos], Some(&selector[pos + 1..]))
        } else {
            (selector, None)
        };

        let script = if let Some(attr_name) = attr {
            format!(
                r#"(function(){{ let el = $("{}"); return el ? attr(el, "{}") : null; }})()"#,
                sel.replace('"', r#"\""#),
                attr_name.replace('"', r#"\""#)
            )
        } else {
            format!(
                r#"(function(){{ let el = $("{}"); return el ? text(el) : ""; }})()"#,
                sel.replace('"', r#"\""#)
            )
        };

        self.runtime
            .execute(&self.html, &script, None)
            .map(|s| s.trim_matches('"').to_string())
            .map_err(RuleError::JsError)
    }

    fn eval_js(&mut self, script: &str) -> Result<String, RuleError> {
        self.runtime
            .execute(&self.html, script, None)
            .map(|s| {
                if s.starts_with('"') && s.ends_with('"') {
                    s[1..s.len() - 1].to_string()
                } else {
                    s
                }
            })
            .map_err(RuleError::JsError)
    }

    fn eval_regex(&self, rule: &str) -> Result<String, RuleError> {
        let (pattern, replacement) = if let Some(pos) = rule.find("##") {
            (&rule[..pos], Some(&rule[pos + 2..]))
        } else {
            (rule, None)
        };

        let re =
            Regex::new(pattern).map_err(|e: regex::Error| RuleError::RegexError(e.to_string()))?;

        if let Some(repl) = replacement {
            Ok(re.replace_all(&self.html, repl).to_string())
        } else if let Some(caps) = re.captures(&self.html) {
            Ok(caps
                .get(1)
                .or(caps.get(0))
                .map(|m| m.as_str())
                .unwrap_or("")
                .to_string())
        } else {
            Ok(String::new())
        }
    }

    fn eval_json(&self, path: &str) -> Result<String, RuleError> {
        // Parse JSON from content (html field stores raw content)
        let json: serde_json::Value = serde_json::from_str(&self.html)
            .map_err(|e| RuleError::JsonPathError(format!("Invalid JSON: {}", e)))?;

        // Use jsonpath_lib
        let result = jsonpath_lib::select(&json, path)
            .map_err(|e| RuleError::JsonPathError(format!("Invalid JSONPath '{}': {}", path, e)))?;

        // Convert result to string
        match result.as_slice() {
            [] => Ok(String::new()),
            [single] => json_value_to_string(single),
            multiple => {
                // Return as JSON array
                let arr: Vec<_> = multiple.iter().collect();
                serde_json::to_string(&arr)
                    .map_err(|e| RuleError::JsonPathError(format!("Serialize error: {}", e)))
            }
        }
    }

    pub fn eval_list(
        &mut self,
        list_sel: &str,
        item_rules: &[(&str, &str)],
    ) -> Result<Vec<std::collections::HashMap<String, String>>, RuleError> {
        let script = format!(r#"$$("{}").length"#, list_sel.replace('"', r#"\""#));
        let count: usize = self
            .runtime
            .execute(&self.html, &script, None)
            .map_err(RuleError::JsError)?
            .parse()
            .unwrap_or(0);

        let mut results = Vec::new();
        for i in 0..count {
            let mut item = std::collections::HashMap::new();
            for (key, rule) in item_rules {
                let rule = rule.replace("{i}", &i.to_string());
                if let Ok(value) = self.eval(&rule) {
                    if !value.is_empty() {
                        item.insert(key.to_string(), value);
                    }
                }
            }
            if !item.is_empty() {
                results.push(item);
            }
        }
        Ok(results)
    }
}

/// Convert JSON value to string representation
fn json_value_to_string(value: &serde_json::Value) -> Result<String, RuleError> {
    match value {
        serde_json::Value::Null => Ok(String::new()),
        serde_json::Value::Bool(b) => Ok(b.to_string()),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::String(s) => Ok(s.clone()),
        _ => serde_json::to_string(value)
            .map_err(|e| RuleError::JsonPathError(format!("Serialize error: {}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_css_selector() {
        let mut parser = RuleParser::new(r#"<div class="title">Hello</div>"#);
        assert_eq!(parser.eval("@css:.title").unwrap(), "Hello");
    }

    #[test]
    fn test_css_attr() {
        let mut parser = RuleParser::new(r#"<a href="https://example.com">Link</a>"#);
        assert_eq!(parser.eval("@css:a@href").unwrap(), "https://example.com");
    }

    #[test]
    fn test_js_expression() {
        let mut parser = RuleParser::new(r#"<div id="test">World</div>"#);
        assert_eq!(parser.eval(r##"@js:text($("#test"))"##).unwrap(), "World");
    }

    #[test]
    fn test_regex() {
        let mut parser = RuleParser::new(r#"<span>ID: 12345</span>"#);
        assert_eq!(parser.eval(r#"@re:ID:\s*(\d+)"#).unwrap(), "12345");
    }

    #[test]
    fn test_webview_error() {
        let mut parser = RuleParser::new("");
        assert!(matches!(
            parser.eval("@webview:test"),
            Err(RuleError::WebViewNotSupported)
        ));
    }

    #[test]
    fn test_json_path_simple() {
        let json = r#"{"title": "遮天", "author": "辰东"}"#;
        let mut parser = RuleParser::new(json);
        assert_eq!(parser.eval("@json:$.title").unwrap(), "遮天");
        assert_eq!(parser.eval("@json:$.author").unwrap(), "辰东");
    }

    #[test]
    fn test_json_path_nested() {
        let json = r#"{"book": {"title": "遮天", "info": {"status": "completed"}}}"#;
        let mut parser = RuleParser::new(json);
        assert_eq!(parser.eval("@json:$.book.title").unwrap(), "遮天");
        assert_eq!(
            parser.eval("@json:$.book.info.status").unwrap(),
            "completed"
        );
    }

    #[test]
    fn test_json_path_array() {
        let json =
            r#"{"chapters": [{"id": "ch1", "title": "第一章"}, {"id": "ch2", "title": "第二章"}]}"#;
        let mut parser = RuleParser::new(json);
        assert_eq!(parser.eval("@json:$.chapters[0].title").unwrap(), "第一章");
        assert_eq!(parser.eval("@json:$.chapters[1].id").unwrap(), "ch2");
    }

    #[test]
    fn test_json_path_wildcard() {
        let json = r#"{"tags": ["玄幻", "修仙", "热血"]}"#;
        let mut parser = RuleParser::new(json);
        let result = parser.eval("@json:$.tags[*]").unwrap();
        assert!(result.contains("玄幻"));
        assert!(result.contains("修仙"));
    }

    #[test]
    fn test_zhetian_json_parse() {
        // 读取遮天书籍 JSON 测试数据
        let json = include_str!("../../tests/rule/fixtures/novel/zhetian_novel_info.json");
        let mut parser = RuleParser::new(json);

        // 测试基本信息解析
        assert_eq!(parser.eval("@json:$.novel.id").unwrap(), "zhetian-001");
        assert_eq!(parser.eval("@json:$.novel.title").unwrap(), "遮天");
        assert_eq!(parser.eval("@json:$.novel.author").unwrap(), "辰东");
        assert_eq!(parser.eval("@json:$.novel.status").unwrap(), "已完结");
        assert_eq!(
            parser.eval("@json:$.novel.cover").unwrap(),
            "https://img.example.com/novels/zhetian/cover.jpg"
        );

        // 测试嵌套结构
        assert_eq!(parser.eval("@json:$.novel.stats.rating").unwrap(), "9.6");

        // 测试数组
        let tags_result = parser.eval("@json:$.novel.tags[*]").unwrap();
        assert!(tags_result.contains("玄幻"));
        assert!(tags_result.contains("修仙"));

        // 测试章节信息
        assert_eq!(parser.eval("@json:$.novel.chapters.total").unwrap(), "1880");
        assert_eq!(
            parser
                .eval("@json:$.novel.chapters.volumes[0].title")
                .unwrap(),
            "第一卷 九龙拉棺"
        );
        assert_eq!(
            parser
                .eval("@json:$.novel.chapters.volumes[0].chapter_list[0].title")
                .unwrap(),
            "第一章 九龙拉棺"
        );
    }
}
