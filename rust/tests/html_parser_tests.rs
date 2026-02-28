//! Integration tests for HTML parsing with JS scripts

use rust_lib_omniread::js_engine::JsRuntime;

const NOVEL_INFO_HTML: &str = include_str!("fixtures/novel_info.html");
const CHAPTER_CONTENT_HTML: &str = include_str!("fixtures/chapter_content.html");

const NOVEL_INFO_SCRIPT: &str = include_str!("fixtures/parse_novel_info.js");
const CHAPTERS_SCRIPT: &str = include_str!("fixtures/parse_chapters.js");
const CHAPTER_CONTENT_SCRIPT: &str = include_str!("fixtures/parse_chapter_content.js");

#[test]
fn test_parse_novel_info() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(NOVEL_INFO_HTML, NOVEL_INFO_SCRIPT, None).unwrap();

    let info: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(info["title"], "Sword Art Online");
    assert_eq!(info["author"], "Kawahara Reki");
}

#[test]
fn test_parse_chapters() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(NOVEL_INFO_HTML, CHAPTERS_SCRIPT, None).unwrap();

    let chapters: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(chapters.len(), 4);
}

#[test]
fn test_parse_chapter_content() {
    let mut rt = JsRuntime::new();
    let result = rt.execute(CHAPTER_CONTENT_HTML, CHAPTER_CONTENT_SCRIPT, None).unwrap();

    let content: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(content["title"], "Chapter 1: The World of Swords");
}
