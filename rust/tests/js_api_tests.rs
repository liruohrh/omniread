//! Unit tests for JS API functions

use rust_lib_omniread::html_parser::JsRuntime;
use std::collections::HashMap;

const TEST_HTML: &str = include_str!("fixtures/test_basic.html");

// $ selector tests

#[test]
fn test_select_by_id() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(TEST_HTML, r##"let el = $("#container"); el ? el.tagName : null"##, None).unwrap();
    assert!(result.contains("div"));
}

#[test]
fn test_select_by_class() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(TEST_HTML, r#"text($(".title"))"#, None).unwrap();
    assert!(result.contains("Hello World"));
}

#[test]
fn test_select_not_found() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(TEST_HTML, r#"$(".nonexistent") === null"#, None).unwrap();
    assert_eq!(result, "true");
}

// $$ selector tests

#[test]
fn test_select_all_multiple() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(TEST_HTML, r#"$$(".item").length"#, None).unwrap();
    assert_eq!(result, "3");
}

#[test]
fn test_select_all_iterate() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(TEST_HTML, r#"
        let els = $$(".item");
        let texts = [];
        for (let i = 0; i < els.length; i++) { texts.push(text(els[i])); }
        texts;
    "#, None).unwrap();
    let arr: Vec<String> = serde_json::from_str(&result).unwrap();
    assert_eq!(arr.len(), 3);
}

// text() tests

#[test]
fn test_text_simple() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(TEST_HTML, r#"text($("h1"))"#, None).unwrap();
    assert!(result.contains("Hello World"));
}

#[test]
fn test_text_with_children() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(TEST_HTML, r##"text($("#list"))"##, None).unwrap();
    assert!(result.contains("Item 1"));
}

// attr() tests

#[test]
fn test_attr_existing() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(TEST_HTML, r#"attr($("a"), "href")"#, None).unwrap();
    assert!(result.contains("https://example.com"));
}

#[test]
fn test_attr_data() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(TEST_HTML, r##"attr($("#container"), "data-id")"##, None).unwrap();
    assert!(result.contains("123"));
}

// hasClass() tests

#[test]
fn test_has_class_true() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(TEST_HTML, r#"hasClass($("h1"), "title")"#, None).unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_has_class_false() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(TEST_HTML, r#"hasClass($("h1"), "nonexistent")"#, None).unwrap();
    assert_eq!(result, "false");
}

// html() tests

#[test]
fn test_html_simple() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(TEST_HTML, r#"html($(".nested"))"#, None).unwrap();
    assert!(result.contains("<span"));
}

// Nested select tests

#[test]
fn test_nested_select() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(TEST_HTML, r##"text($($("#container"), "h1"))"##, None).unwrap();
    assert!(result.contains("Hello World"));
}

#[test]
fn test_nested_select_all() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(TEST_HTML, r##"$$($("#list"), ".item").length"##, None).unwrap();
    assert_eq!(result, "3");
}

// Encoding tests

#[test]
fn test_base64() {
    let mut rt = JsRuntime::new();
    let result = rt.execute("", r#"base64Decode(base64Encode("hello"))"#, None).unwrap();
    assert!(result.contains("hello"));
}

#[test]
fn test_hex() {
    let mut rt = JsRuntime::new();
    let result = rt.execute("", r#"hexDecode(hexEncode("test"))"#, None).unwrap();
    assert!(result.contains("test"));
}

// Crypto tests

#[test]
fn test_md5() {
    let mut rt = JsRuntime::new();
    let result = rt.execute("", r#"md5("hello")"#, None).unwrap();
    assert!(result.contains("5d41402abc4b2a76b9719d911017c592"));
}

#[test]
fn test_sha256() {
    let mut rt = JsRuntime::new();
    let result = rt.execute("", r#"sha256("hello")"#, None).unwrap();
    assert!(result.contains("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"));
}

// Variables test

#[test]
fn test_vars() {
    let mut rt = JsRuntime::new();
    let mut vars = HashMap::new();
    vars.insert("myVar".to_string(), "myValue".to_string());
    let result = rt.execute("", "myVar", Some(&vars)).unwrap();
    assert!(result.contains("myValue"));
}

// setHtml test

#[test]
fn test_set_html() {
    let mut rt = JsRuntime::new();
    let result = rt.execute("", r#"setHtml("<p>Dynamic</p>"); text($("p"))"#, None).unwrap();
    assert!(result.contains("Dynamic"));
}

// log test

#[test]
fn test_log() {
    let mut rt = JsRuntime::new();
    let result = rt.execute("", r#"log("test"); "ok""#, None).unwrap();
    assert!(result.contains("ok"));
}
