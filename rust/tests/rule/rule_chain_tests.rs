//! Rule chain execution tests

use rust_lib_omniread::rule_engine::RuleParser;

#[test]
fn test_rule_chain_execution() {
    // 测试规则数组的链式执行
    let html = r#"
    <div class="novel-info">
        <h1>遮天 - 辰东</h1>
        <p>作者：辰东</p>
        <p>状态：连载中</p>
    </div>
    "#;

    let mut parser = RuleParser::new(html);

    // 规则数组：先提取 h1 标签内容，然后使用正则表达式提取书名
    let rules = vec![
        "@css:.novel-info h1".to_string(),
        "@re:([^-]+) - ".to_string(),
    ];

    let result = parser.eval_rules(&rules).unwrap();
    assert_eq!(result, "遮天");
}

#[test]
fn test_complex_rule_chain() {
    // 测试复杂的规则数组链式执行
    let html = r#"
    <div class="user-info">
        <span class="name">张三</span>
        <span class="id">ID: 123456</span>
    </div>
    "#;

    let mut parser = RuleParser::new(html);

    // 规则数组：先提取 id 标签内容，然后使用正则表达式提取数字 ID
    let rules = vec![
        "@css:.user-info .id".to_string(),
        "@re:ID: (\\d+)".to_string(),
    ];

    let result = parser.eval_rules(&rules).unwrap();
    assert_eq!(result, "123456");
}
