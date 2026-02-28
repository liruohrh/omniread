use rust_lib_omniread::request_config::{RequestConfig, RequestMethod, TemplateEngine};
use std::collections::HashMap;

#[test]
fn test_template_engine_render_string() {
    let mut vars = HashMap::new();
    vars.insert("keyword".to_string(), "test".to_string());
    vars.insert("id".to_string(), "123".to_string());
    
    let result = TemplateEngine::render_string("https://example.com/search?q={{keyword}}&id={{id}}", &vars).unwrap();
    assert_eq!(result, "https://example.com/search?q=test&id=123");
}

#[test]
fn test_template_engine_extract_variables() {
    let vars = TemplateEngine::extract_variables("https://example.com/{{a}}/{{b}}?q={{c}}");
    assert_eq!(vars, vec!["a", "b", "c"]);
}

#[test]
fn test_request_config_default() {
    let config = RequestConfig::new("https://example.com".to_string());
    assert_eq!(config.method, RequestMethod::Get);
    assert_eq!(config.url, "https://example.com");
    assert!(config.headers.is_empty());
    assert!(config.content_type.is_none());
    assert!(config.body.is_none());
}

#[test]
fn test_request_config_with_custom_method() {
    let mut config = RequestConfig::new("https://example.com".to_string());
    config.method = RequestMethod::Post;
    assert_eq!(config.method, RequestMethod::Post);
}

#[test]
fn test_request_config_with_headers() {
    let mut config = RequestConfig::new("https://example.com".to_string());
    config.headers.insert("X-Custom".to_string(), "value".to_string());
    config.headers.insert("Authorization".to_string(), "Bearer token".to_string());
    
    assert_eq!(config.headers.get("X-Custom"), Some(&"value".to_string()));
    assert_eq!(config.headers.get("Authorization"), Some(&"Bearer token".to_string()));
}
