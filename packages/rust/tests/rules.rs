//! core/rules 的加载与自然语言描述。

use std::path::Path;

use quanttide_founder::core::rules::{Artifact, Verb, Workflow};

const PROFILE_RULE: &str = r#"
document_type: profile

title:
  source: first_h1
  fallback: filename

description:
  location: preface
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

/// 共享资产目录（与 Dart 包同源，同一件事只写一处）。
fn shared_assets(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative)
}

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
    assert_eq!(workflow.steps[0].verb, Verb::Scan);
    assert_eq!(workflow.steps[0].description, "按认知关键词定位候选");
    assert_eq!(workflow.steps[1].verb, Verb::Judge);
    assert!(workflow.rules.contains("留 journal"));
}

#[test]
fn workflow_defaults_are_optional() {
    // description/output/rules/actions 可缺省（同 Dart 的 `??`）
    let workflow = Workflow::from_yaml("name: minimal\ninput: journal\n").expect("合法 YAML");
    assert_eq!(workflow.name, "minimal");
    assert!(workflow.description.is_empty());
    assert!(workflow.output.is_empty());
    assert!(workflow.steps.is_empty());
    assert!(workflow.rules.is_empty());
    assert!(workflow.actions.is_empty());
}

#[test]
fn loads_artifact_and_builds_instruction() {
    let rule = Artifact::from_yaml(PROFILE_RULE).expect("合法 YAML");
    assert_eq!(rule.document_type, "profile");
    assert_eq!(rule.sections.split_by.level(), 2);

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
fn loads_criteria_for_classification() {
    // 判据是数据：意图 + 判据 + 正例 / 反例 / 易混例，由 LLM 执行
    let rule = Artifact::from_yaml(PROFILE_RULE).expect("合法 YAML");
    assert_eq!(rule.name, ""); // 没写 name = 不参与分类
    assert!(rule.criteria_instruction().is_none());

    let rule = Artifact::from_file(shared_assets("rules/insight.yaml")).expect("共享 insight.yaml");
    assert_eq!(rule.name, "insight");
    let criteria = rule.criteria_instruction().expect("有判据");
    assert!(criteria.contains("我发现了什么"));
    assert!(criteria.contains("正例:"));
    assert!(criteria.contains("反例:"));
    assert!(criteria.contains("易混例（含裁决）:"));
    // 分级是数据：节标题按 titles 对号
    assert_eq!(rule.grades.len(), 2);
    assert_eq!(rule.grades[0].name, "confirmed");
    assert!(rule.grades[0].titles.contains(&"已确认".to_string()));
}

#[test]
fn missing_rule_sections_render_empty() {
    // 没有 extract / unknown_content 的规则：这两行不出现（Dart 侧为 null 时不打印 extract 行）
    let rule = Artifact::from_yaml(
        "document_type: x\ntitle: {source: first_h1, fallback: filename}\ndescription: {location: preface, meaning: m}\nsections: {split_by: H2}\n",
    )
    .expect("合法 YAML");
    let instruction = rule.instruction();
    assert!(!instruction.contains("提取字段:"));
    assert!(!instruction.contains("未知内容:"));
    // 没写 subsections：不出现「为子节」
    assert!(instruction.contains("章节切分: 按 H2 切分"));
}

#[test]
fn fixed_vocabulary_fails_at_load_time() {
    // type 该是固定取值：split_by / location / verb 加载期校验
    let err = Artifact::from_yaml(
        "document_type: x\ntitle: {source: first_h1, fallback: filename}\ndescription: {location: preface, meaning: m}\nsections: {split_by: h7}\n",
    )
    .expect_err("非法层级应报错");
    assert!(err.to_string().contains("H1"));

    let err = Artifact::from_yaml(
        "document_type: x\ntitle: {source: 首个 H1 标题, fallback: filename}\ndescription: {location: preface, meaning: m}\nsections: {split_by: H2}\n",
    )
    .expect_err("自然语言不是合法取值");
    assert!(err.to_string().contains("first_h1"));
}

#[test]
fn loads_shared_assets_end_to_end() {
    // 规则外部化：共享 YAML 真的被 Rust 读进来
    for name in [
        "insight.yaml",
        "profile.yaml",
        "roadmap.yaml",
        "intention.yaml",
        "observation.yaml",
        "fragment.yaml",
        "packaging.yaml",
    ] {
        Artifact::from_file(shared_assets("rules").join(name))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    for name in ["grade.yaml", "cluster.yaml", "locate.yaml", "classify.yaml"] {
        Workflow::from_file(shared_assets("workflows").join(name))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    }

    // 合并策略的可选决策在 YAML 里
    let cluster = Workflow::from_file(shared_assets("workflows/cluster.yaml")).expect("cluster");
    assert_eq!(cluster.actions.first().unwrap().name, "add");
    assert_eq!(cluster.actions.last().unwrap().name, "crossLink");
    // 分级的决策表在 YAML 里
    let grade = Workflow::from_file(shared_assets("workflows/grade.yaml")).expect("grade");
    assert_eq!(grade.output, vec!["confirmed", "hypothesis"]);
}

#[test]
fn rejects_broken_yaml() {
    assert!(Workflow::from_yaml("name: [unclosed").is_err());
    assert!(Artifact::from_yaml("document_type: 1\n").is_err());
}
