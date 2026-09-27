//! core/rules 的加载与自然语言描述。

use quanttide_founder::core::rules::{Artifact, Workflow};

const PROFILE_RULE: &str = r#"
document_type: profile

title:
  source: "第一个 H1 标题"
  fallback: "文件名去掉 .md"

description:
  location: "H1 之后、首个 H2 之前的段落"
  meaning: "来源说明，描述这份档案从哪来、覆盖什么"

sections:
  split_by: H2
  subsections: H3
  extract:
    - field: name
      from: "H2 标题"
    - field: paragraphs
      from: "节内段落文本，按顺序保留"
    - field: items
      from: "节内列表条目"
      format: "每条提取 name 和 detail"
      patterns:
        - "**名称**：详情"
        - "名称：详情"
        - "纯文本"
  unknown_content: "保留在 paragraphs 中，不丢弃"
"#;

const CLASSIFY_WORKFLOW: &str = r#"
name: classify
description: 三问过滤——判断日志条目的去向
input: journal
output: [insight, profile, roadmap, journal]
steps:
  - scan: 按认知关键词定位候选
  - judge: 三问判断去向，拿不准不收
rules: |
  「想通了什么」→ insight、「我是谁」→ profile、
  「要做什么」→ roadmap、「发生了什么」→ 留 journal。
"#;

#[test]
fn loads_workflow_from_yaml() {
    let workflow = Workflow::from_yaml(CLASSIFY_WORKFLOW).expect("合法 YAML");
    assert_eq!(workflow.name, "classify");
    assert_eq!(workflow.input, "journal");
    assert_eq!(
        workflow.output,
        vec!["insight", "profile", "roadmap", "journal"]
    );
    assert_eq!(workflow.steps.len(), 2);
    assert_eq!(workflow.steps[0].verb, "scan");
    assert_eq!(workflow.steps[0].description, "按认知关键词定位候选");
    assert_eq!(workflow.steps[1].verb, "judge");
    assert!(workflow.rules.contains("留 journal"));
}

#[test]
fn workflow_defaults_are_optional() {
    // description/output/rules 可缺省（同 Dart 的 `??`）
    let workflow = Workflow::from_yaml("name: minimal\ninput: journal\n").expect("合法 YAML");
    assert_eq!(workflow.name, "minimal");
    assert!(workflow.description.is_empty());
    assert!(workflow.output.is_empty());
    assert!(workflow.steps.is_empty());
    assert!(workflow.rules.is_empty());
}

#[test]
fn loads_artifact_and_builds_instruction() {
    let rule = Artifact::from_yaml(PROFILE_RULE).expect("合法 YAML");
    assert_eq!(rule.document_type, "profile");
    assert_eq!(rule.sections.split_by, "H2");

    let instruction = rule.instruction();
    assert!(instruction.contains("文档类型: profile"));
    assert!(instruction.contains("标题: 第一个 H1 标题（兜底: 文件名去掉 .md）"));
    assert!(instruction.contains("说明: H1 之后、首个 H2 之前的段落，含义是来源说明"));
    assert!(instruction.contains("章节切分: 按 H2 切分，H3 为子节"));
    assert!(instruction.contains("提取字段:"));
    assert!(instruction.contains("  - items: 节内列表条目"));
    assert!(instruction.contains("    格式: **名称**：详情 / 名称：详情 / 纯文本"));
    assert!(instruction.contains("未知内容: 保留在 paragraphs 中，不丢弃"));
}

#[test]
fn missing_rule_sections_render_empty() {
    // 没有 extract / unknown_content 的规则：这两行不出现（Dart 侧为 null 时不打印 extract 行）
    let rule = Artifact::from_yaml(
        "document_type: x\ntitle: {source: s, fallback: f}\ndescription: {location: l, meaning: m}\nsections: {split_by: H2}\n",
    )
    .expect("合法 YAML");
    let instruction = rule.instruction();
    assert!(!instruction.contains("提取字段:"));
    assert!(!instruction.contains("未知内容:"));
}

#[test]
fn rejects_broken_yaml() {
    assert!(Workflow::from_yaml("name: [unclosed").is_err());
    assert!(Artifact::from_yaml("document_type: 1\n").is_err());
}
