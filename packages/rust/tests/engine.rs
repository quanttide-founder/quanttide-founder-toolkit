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
title: {source: first_h1, fallback: filename}
description: {location: preface, meaning: 来源说明}
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
    // 说明取哪个位置由规则配置（location: preface）
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
fn rule_based_extractor_skips_description_when_rule_says_none() {
    let rule = Artifact::from_yaml(
        "document_type: x\ntitle: {source: first_h1, fallback: filename}\ndescription: {location: none, meaning: 不要说明}\nsections: {split_by: H2}\n",
    )
    .expect("合法规则");
    let result = RuleBasedExtractor::new()
        .extract(&sample_sections(), &rule)
        .expect("规则提取不失败");
    assert_eq!(result.description, None);
}

#[test]
fn llm_extractor_parses_fenced_json_response() {
    // 响应外面包了 markdown 代码块——quanttide-agent 的 parse_structured_output 负责剥壳
    let llm = json_llm(
        "```json\n{\"title\":\"档案\",\"description\":\"说明\",\"sections\":[{\"name\":\"节\",\"paragraphs\":[\"段\"]}],\"fields\":{\"motif\":\"赶末班车\"}}\n```",
    );
    let rule = Artifact::from_yaml(PROFILE_RULE).expect("合法规则");
    let result = LlmExtractor::new(llm)
        .extract(&sample_sections(), &rule)
        .expect("LLM 响应可解析");

    assert_eq!(result.title.as_deref(), Some("档案"));
    assert_eq!(result.description.as_deref(), Some("说明"));
    assert_eq!(result.sections.len(), 1);
    assert_eq!(result.sections[0]["name"], json!("节"));
    // extract 字段落在 fields 里
    assert_eq!(result.fields["motif"], json!("赶末班车"));
    assert_eq!(result.to_json()["fields"]["motif"], json!("赶末班车"));
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
fn judge_asks_llm_for_structured_decision_and_reason() {
    let llm = json_llm("{\"decision\":\"confirmed\",\"reason\":\"多日重复\"}");
    let engine = Engine::new(llm);
    let judgment = engine
        .judge("多日重复 = 已确认", &[json!("缓存的规律是 TTL 太长")])
        .expect("LLM 调用成功");
    assert_eq!(judgment.decision, "confirmed");
    // reason 必填——判断必须给出理由
    assert_eq!(judgment.reason, "多日重复");
}

#[test]
fn judge_rejects_unstructured_response() {
    // 回纯文本 = 判断失败，不把整段文本当决策
    let engine = Engine::new(demo_llm());
    let err = engine
        .judge("判据", &[json!("条目")])
        .expect_err("非结构化响应应报错");
    assert!(matches!(err, Error::Parse(_)));
}

#[test]
fn judge_rejects_decision_without_reason() {
    let llm = json_llm("{\"decision\":\"confirmed\"}");
    let engine = Engine::new(llm);
    let err = engine
        .judge("判据", &[json!("条目")])
        .expect_err("缺 reason 应报错");
    assert!(err.to_string().contains("缺少 reason"));
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
    let llm = json_llm("{\"decision\":\"keep\",\"reason\":\"照规则判\"}");
    let engine = Engine::new(llm);
    let result = engine
        .run(&workflow, vec![json!("一条日志"), json!("另一条日志")])
        .expect("执行成功");

    // rules 里没有关键词行 → scan 不产出候选；条目原样留给后续步骤
    assert!(result.candidates.is_empty());
    assert_eq!(result.items, vec![json!("一条日志"), json!("另一条日志")]);
    assert_eq!(result.judgments.len(), 1);
    assert_eq!(result.judgments[0].decision, "keep");
}

#[test]
fn run_scans_by_keywords_read_from_rules() {
    // 关键词是数据：从 workflow 的 rules 里读，scan 定位候选但不清空条目
    let workflow = Workflow::from_yaml(
        "name: locate\ninput: journal\nsteps:\n  - scan: 按关键词表匹配\nrules: |\n  认知类关键词：发现、原来。\n",
    )
    .expect("合法 YAML");
    let llm = json_llm("{\"decision\":\"keep\",\"reason\":\"照规则判\"}");
    let engine = Engine::new(llm);
    let result = engine
        .run(
            &workflow,
            vec![json!("今天发现缓存的规律"), json!("没进展")],
        )
        .expect("执行成功");

    assert_eq!(result.candidates.len(), 1);
    assert_eq!(result.candidates[0].text, "今天发现缓存的规律");
    assert_eq!(result.items.len(), 2); // scan 不清空条目
}

#[test]
fn unknown_verb_fails_at_load_time() {
    // 动词是类型：加载期就拒绝，不留到运行期
    let err = Workflow::from_yaml("name: x\ninput: journal\nsteps:\n  - echo: 说一句\n")
        .expect_err("未知动词应在加载期报错");
    assert!(err.to_string().contains("只认 scan / judge / merge"));
}
