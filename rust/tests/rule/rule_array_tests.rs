//! Rule array and YAML format tests

use std::fs;

use rust_lib_omniread::rule_engine::RuleParser;
use yaml_rust::{Yaml, YamlLoader};

#[test]
fn test_rule_array_execution() {
    // 测试规则数组的链式执行
    let html = r#"{"title": "遮天", "author": "辰东"}"#;

    let mut parser = RuleParser::new(html);

    // 规则数组：提取标题
    let rules = vec!["@json:$.title".to_string()];

    let result = parser.eval_rules(&rules).unwrap();
    assert_eq!(result, "遮天");
}

#[test]
fn test_yaml_rule_parsing() {
    // 测试 YAML 格式规则文件的解析
    let yaml_content = fs::read_to_string("tests/fixtures/zhetian_rule.yaml").unwrap();

    // 解析 YAML
    let docs = YamlLoader::load_from_str(&yaml_content).unwrap();
    let doc = &docs[0];

    // 测试基本字段解析
    assert_eq!(doc["id"].as_str().unwrap(), "zhetian-novel");
    assert_eq!(doc["name"].as_str().unwrap(), "遮天小说");
    assert_eq!(doc["base_url"].as_str().unwrap(), "https://example.com");
    assert_eq!(doc["content_type"].as_str().unwrap(), "novel");

    // 测试规则数组解析
    if let Yaml::Array(rules) = &doc["rules"] {
        assert!(!rules.is_empty());
        assert_eq!(rules[0].as_str().unwrap(), "@css:.novel-info-section");
        assert_eq!(rules[2].as_str().unwrap(), "@json:$.title");
    }
}

#[test]
fn test_complex_rule_array() {
    // 测试复杂的规则数组执行
    let html = r#"{"name": "三国演义", "author": "罗贯中", "year": "1494"}"#;

    let mut parser = RuleParser::new(html);

    // 规则数组：提取作者
    let rules = vec!["@json:$.author".to_string()];

    let result = parser.eval_rules(&rules).unwrap();
    assert_eq!(result, "罗贯中");
}
