//! Tests for parse context, shared variables, webview API, and multi-page content

use rust_lib_omniread::html_parser::{
    BookContext, JsRuntime, PageContext, ParseContext, ParseMode, SourceContext,
};
use std::collections::HashMap;

const ANIME_SINGLE_PAGE: &str = include_str!("fixtures/anime_single_page.html");
const PAGINATED_P1: &str = include_str!("fixtures/paginated_content_p1.html");
#[allow(dead_code)]
const PAGINATED_P2: &str = include_str!("fixtures/paginated_content_p2.html");
const PAGINATED_P3: &str = include_str!("fixtures/paginated_content_p3.html");
const CHAPTER_LIST_P1: &str = include_str!("fixtures/chapter_list_p1.html");
const CHAPTER_LIST_P2: &str = include_str!("fixtures/chapter_list_p2.html");

// ========== Context Injection Tests ==========

#[test]
fn test_book_context_injection() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();
    ctx.set_book(BookContext {
        id: "book123".to_string(),
        title: Some("Test Book".to_string()),
        author: Some("Test Author".to_string()),
        cover: Some("https://example.com/cover.jpg".to_string()),
        url: Some("/book/123".to_string()),
        extra: HashMap::new(),
    });

    let result = rt
        .execute_with_context(
            "<html></html>",
            r#"({ id: book.id, title: book.title, author: book.author })"#,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();
    assert_eq!(json["id"], "book123");
    assert_eq!(json["title"], "Test Book");
    assert_eq!(json["author"], "Test Author");
}

#[test]
fn test_source_context_injection() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();
    ctx.source = Some(SourceContext {
        id: "source1".to_string(),
        name: "Test Source".to_string(),
        base_url: "https://example.com".to_string(),
        headers: HashMap::new(),
    });

    let result = rt
        .execute_with_context(
            "<html></html>",
            r#"({ id: source.id, name: source.name, base: source.base_url })"#,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();
    assert_eq!(json["id"], "source1");
    assert_eq!(json["name"], "Test Source");
    assert_eq!(json["base"], "https://example.com");
}

#[test]
fn test_page_context_injection() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();
    ctx.set_page(PageContext {
        index: 1,
        total: Some(3),
        url: Some("/page/2".to_string()),
        next_url: Some("/page/3".to_string()),
        prev_url: Some("/page/1".to_string()),
    });

    let result = rt
        .execute_with_context(
            "<html></html>",
            r#"({ idx: page.index, total: page.total, next: page.next_url })"#,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();
    assert_eq!(json["idx"], 1);
    assert_eq!(json["total"], 3);
    assert_eq!(json["next"], "/page/3");
}

#[test]
fn test_parse_mode_injection() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();
    ctx.mode = ParseMode::BookAndChapters;

    let result = rt
        .execute_with_context("<html></html>", r#"parseMode"#, &ctx)
        .unwrap();

    assert!(result.json.contains("book_and_chapters"));
}

#[test]
fn test_user_vars_injection() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();
    ctx.set_user_var("cookie", "session=abc123");
    ctx.set_user_var("token", "xyz789");

    let result = rt
        .execute_with_context(
            "<html></html>",
            r#"({ cookie: cookie, token: token })"#,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();
    assert_eq!(json["cookie"], "session=abc123");
    assert_eq!(json["token"], "xyz789");
}

// ========== Shared Variables Tests ==========

#[test]
fn test_shared_get_set() {
    let mut rt = JsRuntime::new();
    let ctx = ParseContext::new();

    let result = rt
        .execute_with_context(
            "<html></html>",
            r#"
        sharedSet("key1", "value1");
        sharedSet("key2", "value2");
        ({ k1: sharedGet("key1"), k2: sharedGet("key2"), k3: sharedGet("nonexistent") })
        "#,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();
    assert_eq!(json["k1"], "value1");
    assert_eq!(json["k2"], "value2");
    assert!(json["k3"].is_null());

    // Check shared_updates contains the values
    assert_eq!(
        result.shared_updates.get("key1"),
        Some(&"value1".to_string())
    );
    assert_eq!(
        result.shared_updates.get("key2"),
        Some(&"value2".to_string())
    );
}

#[test]
fn test_shared_vars_persist_across_context() {
    let mut rt = JsRuntime::new();

    // First execution - set shared vars
    let mut ctx1 = ParseContext::new();
    let result1 = rt
        .execute_with_context(
            r##"<html><div id="book" data-id="123"></div></html>"##,
            r##"
        let el = $("#book");
        let id = attr(el, "data-id");
        sharedSet("bookId", id);
        id
        "##,
            &ctx1,
        )
        .unwrap();

    // Transfer shared vars to next context
    ctx1.shared = result1.shared_updates;

    // Second execution - use shared vars
    let result2 = rt
        .execute_with_context("<html></html>", r#"sharedGet("bookId")"#, &ctx1)
        .unwrap();

    assert!(result2.json.contains("123"));
}

// ========== WebView API Tests ==========

#[test]
fn test_webview_simple_url() {
    let mut rt = JsRuntime::new();
    let ctx = ParseContext::new();

    let result = rt
        .execute_with_context(
            "<html></html>",
            r#"webview("https://example.com/dynamic")"#,
            &ctx,
        )
        .unwrap();

    assert!(result.webview_request.is_some());
    let req = result.webview_request.unwrap();
    assert_eq!(req.url, "https://example.com/dynamic");
    assert!(req.js.is_none());
    assert!(req.wait_for.is_none());
}

#[test]
fn test_webview_with_options() {
    let mut rt = JsRuntime::new();
    let ctx = ParseContext::new();

    let result = rt.execute_with_context(
        "<html></html>",
        r##"webview({ url: "https://example.com/spa", js: "document.click()", waitFor: "#loaded" })"##,
        &ctx,
    ).unwrap();

    assert!(result.webview_request.is_some());
    let req = result.webview_request.unwrap();
    assert_eq!(req.url, "https://example.com/spa");
    assert_eq!(req.js, Some("document.click()".to_string()));
    assert_eq!(req.wait_for, Some("#loaded".to_string()));
}

// ========== Multi-page Content Tests ==========

#[test]
fn test_set_next_page() {
    let mut rt = JsRuntime::new();
    let ctx = ParseContext::new();

    let result = rt
        .execute_with_context(
            PAGINATED_P1,
            r##"
        let nextLink = $(".page-next:not(.disabled)");
        if (nextLink) {
            setNextPage(attr(nextLink, "href"));
        }
        text($("#content"))
        "##,
            &ctx,
        )
        .unwrap();

    assert_eq!(
        result.next_page_url,
        Some("/chapter/ch001/page/2".to_string())
    );
    assert!(result.json.contains("first paragraph"));
}

#[test]
fn test_no_next_page_on_last() {
    let mut rt = JsRuntime::new();
    let ctx = ParseContext::new();

    let result = rt
        .execute_with_context(
            PAGINATED_P3,
            r##"
        let nextLink = $(".page-next:not(.disabled)");
        if (nextLink) {
            setNextPage(attr(nextLink, "href"));
        } else {
            setNextPage(null);
        }
        text($("#content"))
        "##,
            &ctx,
        )
        .unwrap();

    assert!(result.next_page_url.is_none());
    assert!(result.json.contains("final page"));
}

// ========== All-in-One Page Tests (Anime Style) ==========

#[test]
fn test_anime_parse_all_mode() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();
    ctx.mode = ParseMode::All;

    let result = rt
        .execute_with_context(
            ANIME_SINGLE_PAGE,
            r##"
        (function() {
            // Parse anime info
            let info = {
                id: attr($(".anime-container"), "data-anime-id"),
                title: text($(".title")),
                altTitle: text($(".alt-title")),
                studio: text($(".studio")),
                description: text($(".description")),
                rating: text($(".rating")),
                tags: []
            };
            
            let tagEls = $$(".tag");
            for (let i = 0; i < tagEls.length; i++) {
                info.tags.push(attr(tagEls[i], "data-tag"));
            }
            
            // Parse episode list
            let episodes = [];
            let epItems = $$(".episode-item");
            for (let i = 0; i < epItems.length; i++) {
                let item = epItems[i];
                episodes.push({
                    id: attr(item, "data-episode-id"),
                    order: parseInt(attr(item, "data-order")),
                    title: text($(item, ".episode-title")),
                    date: text($(item, ".episode-date"))
                });
            }
            
            // Parse current episode content
            let currentEp = $("#current-episode");
            let videoEl = $("#video-element");
            let content = {
                id: attr(currentEp, "data-current-id"),
                title: text($(".current-title")),
                videoUrl: attr(videoEl, "data-src"),
                synopsis: text($(".synopsis"))
            };
            
            return { info: info, episodes: episodes, content: content };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();

    // Verify anime info
    assert_eq!(json["info"]["id"], "mha-001");
    assert_eq!(json["info"]["title"], "My Hero Academia");
    assert_eq!(json["info"]["tags"].as_array().unwrap().len(), 3);

    // Verify episodes
    assert_eq!(json["episodes"].as_array().unwrap().len(), 3);
    assert_eq!(json["episodes"][0]["id"], "ep001");

    // Verify content
    assert_eq!(json["content"]["id"], "ep001");
    assert!(json["content"]["videoUrl"]
        .as_str()
        .unwrap()
        .contains("mha-ep001"));
}

#[test]
fn test_anime_book_only_mode() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();
    ctx.mode = ParseMode::BookDetail;

    let result = rt
        .execute_with_context(
            ANIME_SINGLE_PAGE,
            r#"
        (function() {
            if (parseMode !== "book_detail") {
                return null;
            }
            return {
                id: attr($(".anime-container"), "data-anime-id"),
                title: text($(".title")),
                studio: text($(".studio")),
                description: text($(".description"))
            };
        })()
        "#,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();
    assert_eq!(json["id"], "mha-001");
    assert_eq!(json["title"], "My Hero Academia");
}

// ========== Paginated Chapter List Tests ==========

#[test]
fn test_chapter_list_pagination() {
    let mut rt = JsRuntime::new();

    // Parse first page
    let ctx1 = ParseContext::new();
    let result1 = rt
        .execute_with_context(
            CHAPTER_LIST_P1,
            r#"
        (function() {
            let chapters = [];
            let items = $$(".chapter-item");
            for (let i = 0; i < items.length; i++) {
                let item = items[i];
                chapters.push({
                    id: attr(item, "data-chapter-id"),
                    title: text($(item, ".chapter-title"))
                });
            }
            
            let nextLink = $(".page-next:not(.disabled)");
            if (nextLink) {
                setNextPage(attr(nextLink, "href"));
            }
            
            return chapters;
        })()
        "#,
            &ctx1,
        )
        .unwrap();

    let chapters1: Vec<serde_json::Value> = serde_json::from_str(&result1.json).unwrap();
    assert_eq!(chapters1.len(), 3);
    assert_eq!(
        result1.next_page_url,
        Some("/book/book123/chapters?page=2".to_string())
    );

    // Parse second page
    let ctx2 = ParseContext::new();
    let result2 = rt
        .execute_with_context(
            CHAPTER_LIST_P2,
            r#"
        (function() {
            let chapters = [];
            let items = $$(".chapter-item");
            for (let i = 0; i < items.length; i++) {
                let item = items[i];
                chapters.push({
                    id: attr(item, "data-chapter-id"),
                    title: text($(item, ".chapter-title"))
                });
            }
            
            let nextLink = $(".page-next:not(.disabled)");
            if (nextLink) {
                setNextPage(attr(nextLink, "href"));
            }
            
            return chapters;
        })()
        "#,
            &ctx2,
        )
        .unwrap();

    let chapters2: Vec<serde_json::Value> = serde_json::from_str(&result2.json).unwrap();
    assert_eq!(chapters2.len(), 2);
    assert!(result2.next_page_url.is_none()); // No more pages
}

// ========== Combined Scenario Tests ==========

#[test]
fn test_full_workflow_with_shared_vars() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();

    // Step 1: Parse book detail page, save ID to shared
    ctx.mode = ParseMode::BookDetail;
    let result1 = rt
        .execute_with_context(
            r#"<html><div class="book" data-book-id="novel123"><h1>Test Novel</h1></div></html>"#,
            r#"
        (function() {
            let id = attr($(".book"), "data-book-id");
            sharedSet("bookId", id);
            return {
                id: id,
                title: text($("h1"))
            };
        })()
        "#,
            &ctx,
        )
        .unwrap();

    ctx.shared = result1.shared_updates;

    // Step 2: Parse chapter list using shared bookId
    ctx.mode = ParseMode::ChapterList;
    let result2 = rt
        .execute_with_context(
            r#"<html><ul><li class="ch" data-id="ch1">Chapter 1</li></ul></html>"#,
            r#"
        (function() {
            let bookId = sharedGet("bookId");
            let items = $$(".ch");
            let chapters = [];
            for (let i = 0; i < items.length; i++) {
                chapters.push({
                    bookId: bookId,
                    id: attr(items[i], "data-id"),
                    title: text(items[i])
                });
            }
            return chapters;
        })()
        "#,
            &ctx,
        )
        .unwrap();

    let chapters: Vec<serde_json::Value> = serde_json::from_str(&result2.json).unwrap();
    assert_eq!(chapters[0]["bookId"], "novel123");
    assert_eq!(chapters[0]["id"], "ch1");
}
