//! 综合测试 - 覆盖各种内容类型的解析场景
//!
//! 测试场景：
//! 1. 轻小说《刀剑神域》- 图文混排章节内容
//! 2. 动漫《剑来》- 视频/剧集/信息同页，m3u8 视频流
//! 3. 漫画《一拳超人》- 下拉式阅读
//! 4. 小说《遮天》- 信息+目录同页，多页内容

use rust_lib_omniread::js_engine::{
    BookContext, JsRuntime, ParseContext, ParseMode, SourceContext,
};
use std::collections::HashMap;

// ========== 测试资源 ==========

// 刀剑神域（轻小说）
const SAO_NOVEL_INFO: &str = include_str!("fixtures/sao_novel_info.html");
const SAO_CHAPTER_CONTENT: &str = include_str!("fixtures/sao_chapter_content.html");

// 剑来（动漫）
const JIANLAI_ANIME_PAGE: &str = include_str!("fixtures/jianlai_anime_page.html");

// 一拳超人（漫画）
const MANGA_INFO: &str = include_str!("fixtures/manga_info.html");
const MANGA_CHAPTER_SCROLL: &str = include_str!("fixtures/manga_chapter_scroll.html");

// 遮天（小说，信息+目录同页，分页内容）
const ZHETIAN_NOVEL_INFO: &str = include_str!("fixtures/zhetian_novel_info.html");
const ZHETIAN_CHAPTER_P1: &str = include_str!("fixtures/zhetian_chapter_p1.html");
const ZHETIAN_CHAPTER_P2: &str = include_str!("fixtures/zhetian_chapter_p2.html");
const ZHETIAN_CHAPTER_P3: &str = include_str!("fixtures/zhetian_chapter_p3.html");

// ========== 1. 轻小说《刀剑神域》测试 ==========

#[test]
fn test_sao_parse_novel_info() {
    let mut rt = JsRuntime::new();
    let ctx = ParseContext::new();

    let result = rt
        .execute_with_context(
            SAO_NOVEL_INFO,
            r##"
        (function() {
            return {
                id: attr($(".novel-container"), "data-novel-id"),
                title: text($(".title")),
                altTitle: text($(".alt-title")),
                author: text($(".author a")),
                illustrator: text($(".illustrator a")),
                status: attr($(".status"), "data-status"),
                cover: attr($(".cover"), "data-src"),
                description: text($(".description")),
                tags: (function() {
                    let tags = [];
                    let els = $$(".tag");
                    for (let i = 0; i < els.length; i++) {
                        tags.push(attr(els[i], "data-tag"));
                    }
                    return tags;
                })(),
                views: text($(".views")),
                rating: text($(".rating"))
            };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();
    assert_eq!(json["id"], "sao-001");
    assert_eq!(json["title"], "刀剑神域");
    assert_eq!(json["author"], "川原砾");
    assert_eq!(json["status"], "ongoing");
    assert_eq!(json["tags"].as_array().unwrap().len(), 5);
}

#[test]
fn test_sao_parse_chapters_from_novel_page() {
    let mut rt = JsRuntime::new();
    let ctx = ParseContext::new();

    let result = rt
        .execute_with_context(
            SAO_NOVEL_INFO,
            r##"
        (function() {
            let chapters = [];
            let items = $$(".chapter-item");
            for (let i = 0; i < items.length; i++) {
                let item = items[i];
                let link = $(item, "a");
                chapters.push({
                    id: attr(item, "data-chapter-id"),
                    title: text($(item, ".chapter-title")),
                    url: attr(link, "href"),
                    date: text($(item, ".chapter-date"))
                });
            }
            return chapters;
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let chapters: Vec<serde_json::Value> = serde_json::from_str(&result.json).unwrap();
    assert_eq!(chapters.len(), 7);
    assert_eq!(chapters[0]["id"], "sao-01-prologue");
    assert_eq!(chapters[0]["title"], "序章");
}

#[test]
fn test_sao_parse_chapter_content_with_images() {
    let mut rt = JsRuntime::new();
    let ctx = ParseContext::new();

    let result = rt
        .execute_with_context(
            SAO_CHAPTER_CONTENT,
            r##"
        (function() {
            let content = [];
            
            // 解析段落和插图
            let elements = $$("#chapter-content > *");
            for (let i = 0; i < elements.length; i++) {
                let el = elements[i];
                if (el.tagName === "p") {
                    let t = text(el).trim();
                    if (t) {
                        content.push({ type: "text", text: t });
                    }
                } else if (el.tagName === "figure") {
                    let img = $(el, ".illust-img");
                    let caption = $(el, ".illust-caption");
                    content.push({
                        type: "image",
                        url: attr(img, "data-src"),
                        caption: caption ? text(caption).trim() : null
                    });
                }
            }
            
            return {
                chapterId: attr($(".reader-container"), "data-chapter-id"),
                title: text($(".chapter-title")),
                content: content,
                prevChapter: attr($(".nav-prev"), "data-chapter"),
                nextChapter: attr($(".nav-next"), "data-chapter")
            };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();
    assert_eq!(json["chapterId"], "sao-01-ch01");
    assert_eq!(json["title"], "第一章 剑之世界");

    let content = json["content"].as_array().unwrap();
    // 应该有文本和图片混合
    let has_text = content.iter().any(|c| c["type"] == "text");
    let has_image = content.iter().any(|c| c["type"] == "image");
    assert!(has_text);
    assert!(has_image);

    // 验证导航
    assert_eq!(json["prevChapter"], "sao-01-prologue");
    assert_eq!(json["nextChapter"], "sao-01-ch02");
}

// ========== 2. 动漫《剑来》测试 ==========

#[test]
fn test_jianlai_parse_anime_info() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();
    ctx.mode = ParseMode::BookDetail;

    let result = rt
        .execute_with_context(
            JIANLAI_ANIME_PAGE,
            r##"
        (function() {
            return {
                id: attr($(".anime-page"), "data-anime-id"),
                title: text($(".anime-title")),
                subtitle: text($(".anime-subtitle")),
                originalAuthor: text($(".original-author a")),
                studio: text($(".studio a")),
                director: text($(".director")),
                status: attr($(".status"), "data-status"),
                cover: attr($(".cover-img"), "data-src"),
                description: text($(".description-text")),
                genres: (function() {
                    let genres = [];
                    let els = $$(".genre-tag");
                    for (let i = 0; i < els.length; i++) {
                        genres.push(text(els[i]).trim());
                    }
                    return genres;
                })(),
                rating: text($(".rating")),
                followers: text($(".followers"))
            };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();
    assert_eq!(json["id"], "jianlai-s1");
    assert_eq!(json["title"], "剑来 第一季");
    assert_eq!(json["originalAuthor"], "烽火戏诸侯");
    assert_eq!(json["status"], "ongoing");
}

#[test]
fn test_jianlai_parse_episode_list() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();
    ctx.mode = ParseMode::ChapterList;

    let result = rt
        .execute_with_context(
            JIANLAI_ANIME_PAGE,
            r##"
        (function() {
            let episodes = [];
            let items = $$(".episode-item");
            for (let i = 0; i < items.length; i++) {
                let item = items[i];
                let link = $(item, ".episode-link");
                episodes.push({
                    id: attr(item, "data-episode-id"),
                    order: parseInt(attr(item, "data-order")),
                    number: text($(item, ".episode-number")).trim(),
                    title: text($(item, ".episode-name")).trim(),
                    duration: text($(item, ".episode-duration")).trim(),
                    isVip: attr($(item, ".episode-vip"), "data-vip") === "true",
                    url: attr(link, "href")
                });
            }
            return episodes;
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let episodes: Vec<serde_json::Value> = serde_json::from_str(&result.json).unwrap();
    assert_eq!(episodes.len(), 8);
    assert_eq!(episodes[0]["id"], "ep01");
    assert_eq!(episodes[0]["title"], "少年剑客");
    assert_eq!(episodes[3]["isVip"], true);
}

#[test]
fn test_jianlai_parse_video_sources_m3u8() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();
    ctx.mode = ParseMode::Content;

    let result = rt
        .execute_with_context(
            JIANLAI_ANIME_PAGE,
            r##"
        (function() {
            let sources = [];
            let videoSources = $$(".video-source");
            for (let i = 0; i < videoSources.length; i++) {
                let source = videoSources[i];
                sources.push({
                    quality: attr(source, "data-quality"),
                    url: attr(source, "src"),
                    type: attr(source, "type")
                });
            }
            
            return {
                episodeId: attr($(".anime-page"), "data-current-episode"),
                title: text($(".episode-title")),
                synopsis: text($(".episode-synopsis")),
                poster: attr($("#video-player"), "poster"),
                sources: sources,
                duration: text($(".duration")),
                releaseDate: text($(".release-date"))
            };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();
    assert_eq!(json["episodeId"], "ep01");
    assert_eq!(json["title"], "第1集 少年剑客");

    let sources = json["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 3);
    assert!(sources[0]["url"].as_str().unwrap().contains(".m3u8"));
    assert_eq!(sources[0]["type"], "application/x-mpegURL");
}

#[test]
fn test_jianlai_parse_all_in_one_page() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();
    ctx.mode = ParseMode::All;

    let result = rt
        .execute_with_context(
            JIANLAI_ANIME_PAGE,
            r##"
        (function() {
            // 动漫信息
            let info = {
                id: attr($(".anime-page"), "data-anime-id"),
                title: text($(".anime-title")),
                studio: text($(".studio a"))
            };
            
            // 剧集列表
            let episodes = [];
            let items = $$(".episode-item");
            for (let i = 0; i < items.length; i++) {
                episodes.push({
                    id: attr(items[i], "data-episode-id"),
                    title: text($(items[i], ".episode-name")).trim()
                });
            }
            
            // 当前视频
            let video = {
                episodeId: attr($(".anime-page"), "data-current-episode"),
                source: attr($$(".video-source")[0], "src")
            };
            
            return { info: info, episodes: episodes, video: video };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();
    assert_eq!(json["info"]["id"], "jianlai-s1");
    assert_eq!(json["episodes"].as_array().unwrap().len(), 8);
    assert!(json["video"]["source"].as_str().unwrap().contains(".m3u8"));
}

// ========== 3. 漫画《一拳超人》测试 ==========

#[test]
fn test_manga_parse_info() {
    let mut rt = JsRuntime::new();
    let ctx = ParseContext::new();

    let result = rt
        .execute_with_context(
            MANGA_INFO,
            r##"
        (function() {
            return {
                id: attr($(".manga-container"), "data-manga-id"),
                title: text($(".manga-title")),
                subtitle: text($(".manga-subtitle")),
                author: text($(".author a")),
                artist: text($(".artist a")),
                status: attr($(".status"), "data-status"),
                cover: attr($(".cover"), "data-src"),
                description: text($(".description")),
                tags: (function() {
                    let tags = [];
                    let els = $$(".tag");
                    for (let i = 0; i < els.length; i++) {
                        tags.push(attr(els[i], "data-tag"));
                    }
                    return tags;
                })(),
                views: text($(".views")),
                rating: text($(".rating"))
            };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();
    assert_eq!(json["id"], "opm-001");
    assert_eq!(json["title"], "一拳超人");
    assert_eq!(json["author"], "ONE");
    assert_eq!(json["artist"], "村田雄介");
}

#[test]
fn test_manga_parse_chapters() {
    let mut rt = JsRuntime::new();
    let ctx = ParseContext::new();

    let result = rt
        .execute_with_context(
            MANGA_INFO,
            r##"
        (function() {
            let chapters = [];
            let items = $$(".chapter-item");
            for (let i = 0; i < items.length; i++) {
                let item = items[i];
                let link = $(item, "a");
                chapters.push({
                    id: attr(item, "data-chapter-id"),
                    number: text($(item, ".chapter-num")).trim(),
                    title: text($(item, ".chapter-title")).trim(),
                    date: text($(item, ".chapter-date")).trim(),
                    url: attr(link, "href"),
                    isNew: hasClass(item, "new")
                });
            }
            return chapters;
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let chapters: Vec<serde_json::Value> = serde_json::from_str(&result.json).unwrap();
    assert!(chapters.len() > 0);
    assert_eq!(chapters[0]["id"], "ch245");
    assert_eq!(chapters[0]["isNew"], true);
}

#[test]
fn test_manga_parse_scroll_reader_images() {
    let mut rt = JsRuntime::new();
    let ctx = ParseContext::new();

    let result = rt
        .execute_with_context(
            MANGA_CHAPTER_SCROLL,
            r##"
        (function() {
            let images = [];
            let pages = $$(".manga-page");
            for (let i = 0; i < pages.length; i++) {
                let page = pages[i];
                let img = $(page, ".page-img");
                images.push({
                    page: Number(attr(page, "data-page")),
                    url: attr(img, "data-src"),
                    width: Number(attr(img, "data-width")),
                    height: Number(attr(img, "data-height"))
                });
            }
            
            return {
                mangaId: attr($(".reader-container"), "data-manga-id"),
                chapterId: attr($(".reader-container"), "data-chapter-id"),
                title: text($(".chapter-title")),
                images: images,
                totalPages: images.length,
                prevChapter: attr($(".nav-prev"), "data-chapter"),
                nextChapter: attr($(".nav-next"), "data-chapter")
            };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();
    assert_eq!(json["mangaId"], "opm-001");
    assert_eq!(json["chapterId"], "ch001");
    assert_eq!(json["title"], "第1话 一拳");
    assert_eq!(json["totalPages"], 12);

    let images = json["images"].as_array().unwrap();
    assert_eq!(images.len(), 12);
    assert!(images[0]["url"].as_str().unwrap().contains("/001.jpg"));
    assert_eq!(images[0]["width"].as_f64().unwrap() as i32, 844);
}

// ========== 4. 小说《遮天》测试 ==========

#[test]
fn test_zhetian_parse_novel_info_and_chapters_same_page() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();
    ctx.mode = ParseMode::BookAndChapters;

    let result = rt
        .execute_with_context(
            ZHETIAN_NOVEL_INFO,
            r##"
        (function() {
            // 解析小说信息
            let info = {
                id: attr($(".novel-page"), "data-novel-id"),
                title: text($(".novel-title")),
                subtitle: text($(".novel-subtitle")),
                author: text($(".author a")),
                status: attr($(".status"), "data-status"),
                cover: attr($(".novel-cover"), "data-src"),
                wordCount: text($(".word-count")),
                description: text($(".intro-content")),
                tags: (function() {
                    let tags = [];
                    let els = $$(".tag");
                    for (let i = 0; i < els.length; i++) {
                        tags.push(attr(els[i], "data-tag"));
                    }
                    return tags;
                })()
            };
            
            // 解析章节列表（同一页面）
            let chapters = [];
            let items = $$(".chapter-item");
            for (let i = 0; i < items.length; i++) {
                let item = items[i];
                let link = $(item, "a");
                chapters.push({
                    id: attr(item, "data-chapter-id"),
                    title: text($(item, ".chapter-title")).trim(),
                    date: text($(item, ".update-time")).trim(),
                    url: attr(link, "href"),
                    isNew: hasClass(item, "new")
                });
            }
            
            return { info: info, chapters: chapters };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();

    // 验证小说信息
    assert_eq!(json["info"]["id"], "zhetian-001");
    assert_eq!(json["info"]["title"], "遮天");
    assert_eq!(json["info"]["author"], "辰东");
    assert_eq!(json["info"]["status"], "completed");

    // 验证章节列表
    let chapters = json["chapters"].as_array().unwrap();
    assert!(chapters.len() > 0);
    assert_eq!(chapters[0]["id"], "ch0001");
    assert_eq!(chapters[0]["title"], "第一章 九龙拉棺");
}

#[test]
fn test_zhetian_parse_paginated_content() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();
    ctx.mode = ParseMode::Content;

    // 解析第一页
    let result1 = rt
        .execute_with_context(
            ZHETIAN_CHAPTER_P1,
            r##"
        (function() {
            let paragraphs = [];
            let ps = $$("#chapter-content p");
            for (let i = 0; i < ps.length; i++) {
                let t = text(ps[i]).trim();
                if (t) paragraphs.push(t);
            }
            
            let container = $(".reader-container");
            let currentPage = Number(attr(container, "data-page"));
            let totalPages = Number(attr(container, "data-total-pages"));
            
            // 检查是否有下一页
            let nextLink = $(".page-next:not(.disabled)");
            if (nextLink) {
                setNextPage(attr(nextLink, "href"));
            }
            
            return {
                chapterId: attr(container, "data-chapter-id"),
                title: text($(".chapter-title")),
                currentPage: currentPage,
                totalPages: totalPages,
                content: paragraphs
            };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let json1: serde_json::Value = serde_json::from_str(&result1.json).unwrap();
    assert_eq!(json1["chapterId"], "ch0001");
    assert_eq!(json1["currentPage"].as_f64().unwrap() as i32, 1);
    assert_eq!(json1["totalPages"].as_f64().unwrap() as i32, 3);
    assert_eq!(
        result1.next_page_url,
        Some("/novel/zhetian-001/chapter/ch0001?page=2".to_string())
    );

    // 解析第二页
    let result2 = rt
        .execute_with_context(
            ZHETIAN_CHAPTER_P2,
            r##"
        (function() {
            let paragraphs = [];
            let ps = $$("#chapter-content p");
            for (let i = 0; i < ps.length; i++) {
                let t = text(ps[i]).trim();
                if (t) paragraphs.push(t);
            }
            
            let container = $(".reader-container");
            let nextLink = $(".page-next:not(.disabled)");
            if (nextLink) {
                setNextPage(attr(nextLink, "href"));
            }
            
            return {
                currentPage: Number(attr(container, "data-page")),
                content: paragraphs
            };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let json2: serde_json::Value = serde_json::from_str(&result2.json).unwrap();
    assert_eq!(json2["currentPage"].as_f64().unwrap() as i32, 2);
    assert_eq!(
        result2.next_page_url,
        Some("/novel/zhetian-001/chapter/ch0001?page=3".to_string())
    );

    // 解析第三页（最后一页）
    let result3 = rt
        .execute_with_context(
            ZHETIAN_CHAPTER_P3,
            r##"
        (function() {
            let paragraphs = [];
            let ps = $$("#chapter-content p");
            for (let i = 0; i < ps.length; i++) {
                let t = text(ps[i]).trim();
                if (t) paragraphs.push(t);
            }
            
            let container = $(".reader-container");
            let nextLink = $(".page-next:not(.disabled)");
            if (nextLink) {
                setNextPage(attr(nextLink, "href"));
            } else {
                setNextPage(null);
            }
            
            return {
                currentPage: Number(attr(container, "data-page")),
                content: paragraphs
            };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let json3: serde_json::Value = serde_json::from_str(&result3.json).unwrap();
    assert_eq!(json3["currentPage"].as_f64().unwrap() as i32, 3);
    assert!(result3.next_page_url.is_none()); // 没有下一页
}

// ========== 5. 共享变量跨页面测试 ==========

#[test]
fn test_shared_vars_across_parsing_steps() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();

    // 步骤1：从小说信息页解析 ID 并保存到共享变量
    ctx.mode = ParseMode::BookDetail;
    let result1 = rt
        .execute_with_context(
            ZHETIAN_NOVEL_INFO,
            r##"
        (function() {
            let novelId = attr($(".novel-page"), "data-novel-id");
            let title = text($(".novel-title"));
            
            // 保存到共享变量
            sharedSet("novelId", novelId);
            sharedSet("novelTitle", title);
            
            return { id: novelId, title: title };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    // 更新上下文中的共享变量
    ctx.shared = result1.shared_updates;

    // 步骤2：在章节内容页使用共享变量
    ctx.mode = ParseMode::Content;
    let result2 = rt
        .execute_with_context(
            ZHETIAN_CHAPTER_P1,
            r##"
        (function() {
            // 读取共享变量
            let novelId = sharedGet("novelId");
            let novelTitle = sharedGet("novelTitle");
            
            return {
                novelId: novelId,
                novelTitle: novelTitle,
                chapterId: attr($(".reader-container"), "data-chapter-id"),
                chapterTitle: text($(".chapter-title"))
            };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result2.json).unwrap();
    assert_eq!(json["novelId"], "zhetian-001");
    assert_eq!(json["novelTitle"], "遮天");
    assert_eq!(json["chapterId"], "ch0001");
}

// ========== 6. 上下文对象注入测试 ==========

#[test]
fn test_context_injection_with_book_info() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();

    // 设置书籍上下文
    ctx.set_book(BookContext {
        id: "zhetian-001".to_string(),
        title: Some("遮天".to_string()),
        author: Some("辰东".to_string()),
        cover: None,
        url: Some("/novel/zhetian-001".to_string()),
        extra: HashMap::new(),
    });

    // 设置数据源上下文
    ctx.source = Some(SourceContext {
        id: "example-source".to_string(),
        name: "示例书源".to_string(),
        base_url: "https://example.com".to_string(),
        headers: HashMap::new(),
    });

    let result = rt
        .execute_with_context(
            ZHETIAN_CHAPTER_P1,
            r##"
        (function() {
            return {
                // 从上下文读取书籍信息
                bookId: book ? book.id : null,
                bookTitle: book ? book.title : null,
                bookAuthor: book ? book.author : null,
                // 从上下文读取数据源信息
                sourceId: source ? source.id : null,
                sourceName: source ? source.name : null,
                sourceBaseUrl: source ? source.base_url : null,
                // 从 HTML 读取章节信息
                chapterId: attr($(".reader-container"), "data-chapter-id"),
                chapterTitle: text($(".chapter-title"))
            };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();
    assert_eq!(json["bookId"], "zhetian-001");
    assert_eq!(json["bookTitle"], "遮天");
    assert_eq!(json["bookAuthor"], "辰东");
    assert_eq!(json["sourceId"], "example-source");
    assert_eq!(json["sourceName"], "示例书源");
}

// ========== 7. 用户变量测试 ==========

#[test]
fn test_user_vars_cookie_injection() {
    let mut rt = JsRuntime::new();
    let mut ctx = ParseContext::new();

    // 模拟用户设置的 cookie
    ctx.set_user_var("cookie", "session_id=abc123; user_token=xyz789");
    ctx.set_user_var("quality", "1080p");

    let result = rt
        .execute_with_context(
            JIANLAI_ANIME_PAGE,
            r##"
        (function() {
            return {
                hasCookie: typeof cookie !== "undefined" && cookie !== null,
                cookieValue: cookie,
                quality: quality,
                animeId: attr($(".anime-page"), "data-anime-id")
            };
        })()
        "##,
            &ctx,
        )
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result.json).unwrap();
    assert_eq!(json["hasCookie"], true);
    assert_eq!(json["cookieValue"], "session_id=abc123; user_token=xyz789");
    assert_eq!(json["quality"], "1080p");
}
