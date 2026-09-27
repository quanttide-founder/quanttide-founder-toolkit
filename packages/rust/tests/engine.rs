//! core/engine：提取器（规则降级 / LLM 填表）与三个动词。
//!
//! LLM 走 quanttide-agent 的 `LLM`，测试用注入的演示客户端，不发网络请求。

mod common;

use common::{demo_llm, json_llm};
use quanttide_founder::Error;
use quanttide_founder::core::engine::{
    Engine, LlmExtractor, RuleBasedExtractor, SemanticExtractor,
};
use quanttide_founder::core::parse::{MarkdownDocument, split_sections};
use quanttide_founder::core::rules::{Artifact, Workflow};
use serde_json::{Value, json};

const PROFILE_RULE: &str = r#"
document_type: profile
title: {source: 第一个 H1 标题, fallback: 文件名}
description: {location: 首个 H2 之前的段落, meaning: 来源说明}
sections:
  split_by: H2
  subsections: H3
  extract:
    - field: items
      from: 节内列表条目
      patterns: ["**名称**：详情"]
  unknown_content: 保留在 paragraphs 中
"#;

const SAMPLE_DOC: &str =
    "# 档案\n\n来自长期观察的档案。\n\n## 工作方式\n\n- **决策快**：信息不全也先拍板\n- 习惯晚睡\n";

fn sample_sections() -> Vec<quanttide_founder::RawSection> {
    let doc = MarkdownDocument::parse("profile.md", SAMPLE_DOC);
    split_sections(&doc.blocks, 2)
}

#[test]
fn rule_based_extractor_fills_sections() {
    let rule = Artifact::from_yaml(PROFILE_RULE).expect("合法规则");
    let result = RuleBasedExtractor::new()
        .extract(&sample_sections(), &rule)
        .expect("规则提取不失败");

    // 规则降级不猜标题
    assert_eq!(result.title, None);
    assert_eq!(result.description.as_deref(), Some("来自长期观察的档案。"));

    assert_eq!(result.sections.len(), 1);
    let section = &result.sections[0];
    assert_eq!(section["name"], json!("工作方式"));
    assert_eq!(
        section["items"],
        json!([
            {"name": "决策快", "detail": "信息不全也先拍板"},
            {"name": "习惯晚睡", "detail": ""}
        ])
    );
    assert!(section.get("paragraphs").is_none());

    let json = result.to_json();
    assert_eq!(json["title"], Value::Null);
    assert_eq!(json["sections"][0]["name"], json!("工作方式"));
}

#[test]
fn llm_extractor_parses_fenced_json_response() {
    // 响应外面包了 markdown 代码块——quanttide-agent 的 parse_structured_output 负责剥壳
    let llm = json_llm(
        "```json\n{\"title\":\"档案\",\"description\":\"说明\",\"sections\":[{\"name\":\"节\",\"paragraphs\":[\"段\"]}]}\n```",
    );
    let rule = Artifact::from_yaml(PROFILE_RULE).expect("合法规则");
    let result = LlmExtractor::new(llm)
        .extract(&sample_sections(), &rule)
        .expect("LLM 响应可解析");

    assert_eq!(result.title.as_deref(), Some("档案"));
    assert_eq!(result.description.as_deref(), Some("说明"));
    assert_eq!(result.sections.len(), 1);
    assert_eq!(result.sections[0]["name"], json!("节"));
}

#[test]
fn llm_extractor_rejects_response_without_json() {
    let llm = json_llm("这题我不会");
    let rule = Artifact::from_yaml(PROFILE_RULE).expect("合法规则");
    let err = LlmExtractor::new(llm)
        .extract(&sample_sections(), &rule)
        .expect_err("没有 JSON 应当报错");
    assert!(matches!(err, Error::Parse(_)));
}

#[test]
fn judge_asks_llm_and_returns_decision() {
    let engine = Engine::new(demo_llm());
    let judgment = engine
        .judge("多日重复 = 已确认", &[json!("缓存的规律是 TTL 太长")])
        .expect("LLM 调用成功");
    assert_eq!(judgment.decision, "（演示模式，未调用 LLM）");
    assert!(judgment.reason.is_none());
}

#[test]
fn scan_locates_candidates_by_keyword() {
    let items = vec![json!("今天没进展"), json!("发现缓存失效的规律")];
    let candidates = Engine::scan(&items, &["发现".to_string()]);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].source, "1");
    assert_eq!(candidates[0].text, "发现缓存失效的规律");

    assert!(Engine::scan(&items, &["不存在的词".to_string()]).is_empty());
}

#[test]
fn merge_appends_incoming() {
    let merged = Engine::merge(&[json!(1), json!(2)], &[json!(3)]);
    assert_eq!(merged, vec![json!(1), json!(2), json!(3)]);
}

#[test]
fn run_walks_workflow_steps_in_order() {
    let workflow = Workflow::from_yaml(
        "name: classify\ninput: journal\nsteps:\n  - scan: 定位\n  - judge: 判断\nrules: 三问\n",
    )
    .expect("合法 YAML");
    let engine = Engine::new(demo_llm());
    let result = engine
        .run(&workflow, vec![json!("一条日志"), json!("另一条日志")])
        .expect("执行成功");

    // scan 步的关键词表是占位空表（同 Dart），因此不产出候选
    assert!(result.items.is_empty());
    assert_eq!(result.judgments.len(), 1);
    assert_eq!(result.judgments[0].decision, "（演示模式，未调用 LLM）");
}

#[test]
fn run_rejects_unknown_verb() {
    let workflow = Workflow::from_yaml("name: x\ninput: journal\nsteps:\n  - echo: 说一句\n")
        .expect("合法 YAML");
    let engine = Engine::new(demo_llm());
    let err = engine.run(&workflow, vec![]).expect_err("未知动词应报错");
    assert!(matches!(err, Error::UnknownVerb(ref verb) if verb == "echo"));
    assert!(err.to_string().contains("只认 scan / judge / merge"));
}
