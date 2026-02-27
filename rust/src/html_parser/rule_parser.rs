//! Rule parser - evaluates selectors against HTML
//!
//! Selector format:
//! - `@css:.selector` - CSS selector, get text
//! - `@css:.selector@attr` - CSS selector, get attribute
//! - `@js:script` - JavaScript expression
//! - `@re:pattern` - Regex on raw HTML
//! - `@re:pattern##replacement` - Regex replace
//! - Plain text - literal value

use regex::Regex;

use super::JsRuntime;

/// Rule evaluation error
#[derive(Debug)]
pub enum RuleError {
    InvalidSelector(String),
    JsError(String),
    RegexError(String),
    WebViewNotSupported,
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSelector(s) => write!(f, "Invalid selector: {}", s),
            Self::JsError(s) => write!(f, "JS error: {}", s),
            Self::RegexError(s) => write!(f, "Regex error: {}", s),
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
        } else if rule.starts_with("@webview:") {
            Err(RuleError::WebViewNotSupported)
        } else if rule.starts_with(".") || rule.starts_with("#") || rule.contains(" ") {
            self.eval_css(rule)
        } else {
            Ok(rule.to_string())
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
}
