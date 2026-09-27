//! core/parse 的行为钉子（对照 Dart 包 test/quanttide_founder_test.dart）。

use quanttide_founder::core::parse::{
    BlockType, MarkdownDocument, parse_named_item, split_sections,
};

#[test]
fn parses_title_and_paragraph() {
    let doc = MarkdownDocument::parse("test.md", "# 标题\n\n段落内容");
    assert_eq!(doc.title(), Some("标题"));
    assert_eq!(doc.blocks.len(), 2);
    assert_eq!(doc.blocks[1].kind, BlockType::Paragraph);
    assert_eq!(doc.blocks[1].text, "段落内容");
}

#[test]
fn missing_h1_has_no_title() {
    let doc = MarkdownDocument::parse("test.md", "## 二级\n\n段落");
    assert_eq!(doc.title(), None);
}

#[test]
fn splits_sections_by_h2_with_preface() {
    let doc = MarkdownDocument::parse("test.md", "前言\n\n## 一节\n\n内容\n\n## 二节");
    let sections = split_sections(&doc.blocks, 2);
    assert_eq!(sections.len(), 3); // 前言 + 两节
    assert_eq!(sections[0].title, "");
    assert_eq!(sections[1].title, "一节");
    assert_eq!(sections[2].title, "二节");
    assert_eq!(sections[1].blocks.len(), 1);
    assert_eq!(sections[1].blocks[0].text, "内容");
}

#[test]
fn h3_becomes_subsection() {
    let doc = MarkdownDocument::parse("test.md", "# t\n\n## 节\n\n### 子节\n\n- 条目\n\n正文");
    let sections = split_sections(&doc.blocks, 2);
    assert_eq!(sections[1].subsections.len(), 1);
    assert_eq!(sections[1].subsections[0].title, "子节");
    // 条目属于子节，正文在子节之后仍归子节（子节直到下一个同级标题）
    assert_eq!(sections[1].subsections[0].blocks.len(), 2);
}

#[test]
fn parses_bullets_and_separators() {
    let doc = MarkdownDocument::parse("t.md", "- 要点一\n* 要点二\n3. 要点三\n\n---\n\n段落");
    let kinds: Vec<BlockType> = doc.blocks.iter().map(|b| b.kind).collect();
    assert_eq!(
        kinds,
        vec![
            BlockType::Bullet,
            BlockType::Bullet,
            BlockType::Bullet,
            BlockType::Separator,
            BlockType::Paragraph,
        ]
    );
    assert_eq!(doc.blocks[2].text, "要点三");
}

#[test]
fn crlf_lines_are_handled() {
    let doc = MarkdownDocument::parse("t.md", "# 标题\r\n\r\n段落\r\n");
    assert_eq!(doc.title(), Some("标题"));
    assert_eq!(doc.blocks[1].text, "段落");
}

#[test]
fn named_item_bold_and_colon_variants() {
    // 加粗名称 + 冒号
    let named = parse_named_item("**决策快**：信息不全也先拍板");
    assert_eq!(
        (named.name.as_str(), named.detail.as_str()),
        ("决策快", "信息不全也先拍板")
    );

    // 普通名称 + 全角冒号
    let named = parse_named_item("单点登录可能是瓶颈：待验证");
    assert_eq!(
        (named.name.as_str(), named.detail.as_str()),
        ("单点登录可能是瓶颈", "待验证")
    );

    // 半角冒号
    let named = parse_named_item("todo: 写周报");
    assert_eq!(
        (named.name.as_str(), named.detail.as_str()),
        ("todo", "写周报")
    );

    // 无冒号：整句作 name
    let named = parse_named_item("统一用一个 CI");
    assert_eq!(
        (named.name.as_str(), named.detail.as_str()),
        ("统一用一个 CI", "")
    );

    // 首字符是冒号：不切分（同 Dart `index > 0`）
    let named = parse_named_item(": 没有名字");
    assert_eq!(
        (named.name.as_str(), named.detail.as_str()),
        (": 没有名字", "")
    );
}
