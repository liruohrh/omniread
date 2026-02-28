//! Rule parser tests for different content types

use std::fs;

use rust_lib_omniread::html_parser::RuleParser;
use rust_lib_omniread::html_parser::Source;
use serde_yaml;

#[test]
fn test_zhetian_novel_rule() {
    // 读取测试数据和规则
    let html = fs::read_to_string("tests/fixtures/zhetian_novel_info.html").unwrap();
    let yaml_content = fs::read_to_string("tests/fixtures/zhetian_rule.yaml").unwrap();
    let source: Source = serde_yaml::from_str(&yaml_content).unwrap();

    let mut parser = RuleParser::new(&html);

    // 测试书籍信息解析
    if let Some(book_detail) = &source.book_detail {
        let title = parser.eval_rules(&book_detail.title).unwrap();
        assert_eq!(title, "遮天");

        let author = parser.eval_rules(&book_detail.author).unwrap();
        assert_eq!(author, "辰东");

        if let Some(cover_rule) = &book_detail.cover {
            let cover = parser.eval_rules(cover_rule).unwrap();
            assert_eq!(cover, "https://img.example.com/novels/zhetian/cover.jpg");
        }
    }

    // 测试章节列表解析
    if let Some(chapter_list) = &source.chapter_list {
        let list_sel = &chapter_list.list[0]; // 使用第一个规则作为列表选择器
        if let Some(url_pattern) = &chapter_list.url_pattern {
            let item_rules: Vec<(&str, &str)> = vec![
                ("id", &chapter_list.id[0]),
                ("title", &chapter_list.title[0]),
                ("url", &url_pattern[0]),
            ];

            let chapters = parser.eval_list(list_sel, &item_rules).unwrap();
            assert!(!chapters.is_empty());
            assert!(chapters[0].contains_key("id"));
            assert!(chapters[0].contains_key("title"));
            assert!(chapters[0].contains_key("url"));
        }
    }
}

#[test]
fn test_sao_light_novel_rule() {
    // 读取测试数据和规则
    let html = fs::read_to_string("tests/fixtures/sao_novel_info.html").unwrap();
    let yaml_content = fs::read_to_string("tests/fixtures/sao_rule.yaml").unwrap();
    let source: Source = serde_yaml::from_str(&yaml_content).unwrap();

    let mut parser = RuleParser::new(&html);

    // 测试书籍信息解析
    if let Some(book_detail) = &source.book_detail {
        let title = parser.eval_rules(&book_detail.title).unwrap();
        assert_eq!(title, "刀剑神域");

        let author = parser.eval_rules(&book_detail.author).unwrap();
        assert_eq!(author, "川原砾");

        if let Some(cover_rule) = &book_detail.cover {
            let cover = parser.eval_rules(cover_rule).unwrap();
            assert_eq!(cover, "https://img.example.com/covers/sao/cover.jpg");
        }
    }

    // 测试章节列表解析
    if let Some(chapter_list) = &source.chapter_list {
        let list_sel = &chapter_list.list[0]; // 使用第一个规则作为列表选择器
        if let Some(url_pattern) = &chapter_list.url_pattern {
            let item_rules: Vec<(&str, &str)> = vec![
                ("id", &chapter_list.id[0]),
                ("title", &chapter_list.title[0]),
                ("url", &url_pattern[0]),
            ];

            let chapters = parser.eval_list(list_sel, &item_rules).unwrap();
            assert!(!chapters.is_empty());
        }
    }
}

#[test]
fn test_jianlai_anime_rule() {
    // 读取测试数据和规则
    let html = fs::read_to_string("tests/fixtures/jianlai_anime_page.html").unwrap();
    let yaml_content = fs::read_to_string("tests/fixtures/jianlai_rule.yaml").unwrap();
    let source: Source = serde_yaml::from_str(&yaml_content).unwrap();

    let mut parser = RuleParser::new(&html);

    // 测试动漫信息解析
    if let Some(book_detail) = &source.book_detail {
        let title = parser.eval_rules(&book_detail.title).unwrap();
        assert_eq!(title, "剑来 第一季");

        let author = parser.eval_rules(&book_detail.author).unwrap();
        assert_eq!(author, "烽火戏诸侯");

        if let Some(cover_rule) = &book_detail.cover {
            let cover = parser.eval_rules(cover_rule).unwrap();
            assert_eq!(cover, "https://img.example.com/anime/jianlai/cover.jpg");
        }
    }

    // 测试剧集列表解析
    if let Some(chapter_list) = &source.chapter_list {
        let list_sel = &chapter_list.list[0]; // 使用第一个规则作为列表选择器
        if let Some(url_pattern) = &chapter_list.url_pattern {
            let item_rules: Vec<(&str, &str)> = vec![
                ("id", &chapter_list.id[0]),
                ("title", &chapter_list.title[0]),
                ("url", &url_pattern[0]),
            ];

            let episodes = parser.eval_list(list_sel, &item_rules).unwrap();
            assert!(!episodes.is_empty());
        }
    }

    // 测试视频源解析
    if let Some(content) = &source.content {
        let video_sources = parser.eval_rules(&content.content).unwrap();
        assert!(video_sources.contains("m3u8"));
    }
}

#[test]
fn test_manga_rule() {
    // 读取测试数据和规则
    let html = fs::read_to_string("tests/fixtures/manga_info.html").unwrap();
    let yaml_content = fs::read_to_string("tests/fixtures/manga_rule.yaml").unwrap();
    let source: Source = serde_yaml::from_str(&yaml_content).unwrap();

    let mut parser = RuleParser::new(&html);

    // 测试漫画信息解析
    if let Some(book_detail) = &source.book_detail {
        let title = parser.eval_rules(&book_detail.title).unwrap();
        assert_eq!(title, "一拳超人");

        let author = parser.eval_rules(&book_detail.author).unwrap();
        assert_eq!(author, "ONE");

        if let Some(cover_rule) = &book_detail.cover {
            let cover = parser.eval_rules(cover_rule).unwrap();
            assert_eq!(cover, "https://img.example.com/manga/opm/cover.jpg");
        }
    }

    // 测试章节列表解析
    if let Some(chapter_list) = &source.chapter_list {
        let list_sel = &chapter_list.list[0]; // 使用第一个规则作为列表选择器
        if let Some(url_pattern) = &chapter_list.url_pattern {
            let item_rules: Vec<(&str, &str)> = vec![
                ("id", &chapter_list.id[0]),
                ("title", &chapter_list.title[0]),
                ("url", &url_pattern[0]),
            ];

            let chapters = parser.eval_list(list_sel, &item_rules).unwrap();
            assert!(!chapters.is_empty());
        }
    }
}

#[test]
fn test_json_rule_parsing() {
    // 读取遮天 JSON 测试数据
    let json = fs::read_to_string("tests/fixtures/zhetian_novel_info.json").unwrap();
    let mut parser = RuleParser::new(&json);

    // 测试 JSON Path 解析
    let title = parser.eval("@json:$.novel.title").unwrap();
    assert_eq!(title, "遮天");

    let author = parser.eval("@json:$.novel.author").unwrap();
    assert_eq!(author, "辰东");

    let rating = parser.eval("@json:$.novel.stats.rating").unwrap();
    assert_eq!(rating, "9.6");

    let chapter_title = parser
        .eval("@json:$.novel.chapters.volumes[0].chapter_list[0].title")
        .unwrap();
    assert_eq!(chapter_title, "第一章 九龙拉棺");
}
