//! core/engine：语义提取 + 计算引擎。
//!
//! 提取：按 Artifact 规则从 Section 树提取语义模型（LLM 填表 / 规则降级）。
//! 计算：scan / judge / merge 三个动词，workflow 用它们组装。
//!
//! LLM 调用不自带客户端——直接用 `quanttide-agent` 的 [`LLM`]。

use quanttide_agent::llm::{CompleteOptions, LLM};
use quanttide_agent::message::Message;
use serde_json::{Map, Value};

use crate::core::parse::{BlockType, RawSection, parse_named_item};
use crate::core::rules::{Artifact, DescriptionLocation, Verb, Workflow};
use crate::error::Error;

// ---------------------------------------------------------------------------
// 语义提取
// ---------------------------------------------------------------------------

/// 语义提取结果：标题、说明、章节、字段。
#[derive(Debug, Clone, Default)]
pub struct ExtractionResult {
    pub title: Option<String>,
    pub description: Option<String>,
    pub sections: Vec<Map<String, Value>>,
    /// 规则 `extract` 列出的字段（如母题 / 场景、标题 / 简介 / 立意），由 LLM 按规则填表。
    pub fields: Map<String, Value>,
}

impl ExtractionResult {
    pub fn to_json(&self) -> Value {
        Value::Object(Map::from_iter([
            ("title".to_string(), self.title.clone().into()),
            ("description".to_string(), self.description.clone().into()),
            (
                "sections".to_string(),
                Value::Array(self.sections.iter().cloned().map(Value::Object).collect()),
            ),
            ("fields".to_string(), Value::Object(self.fields.clone())),
        ]))
    }
}

/// 语义提取器：接收 Section 树 + 规则，产出语义模型。
///
/// 提取方式可插拔，主次有别：
/// - [`LlmExtractor`]：LLM 按规则填表——首选
/// - [`RuleBasedExtractor`]：纯规则提取（无 LLM）——降级，不是主实现
pub trait SemanticExtractor {
    fn extract(&self, sections: &[RawSection], rule: &Artifact) -> Result<ExtractionResult, Error>;
}

/// 规则提取器：无 LLM 时的降级方案，按规则配置提取（说明取哪个位置由 `description.location` 定）。
#[derive(Debug, Default, Clone, Copy)]
pub struct RuleBasedExtractor;

impl RuleBasedExtractor {
    pub const fn new() -> Self {
        Self
    }
}

impl SemanticExtractor for RuleBasedExtractor {
    fn extract(&self, sections: &[RawSection], rule: &Artifact) -> Result<ExtractionResult, Error> {
        let result_sections: Vec<Map<String, Value>> =
            sections.iter().skip(1).map(section_to_json).collect();

        // 说明取哪个位置由规则配置；规则没让取就不猜。标题留给 LLM 提取器。
        let description = match rule.description.location {
            DescriptionLocation::Preface => sections.first().map(join_paragraphs),
            DescriptionLocation::Section => sections.get(1).map(join_paragraphs),
            DescriptionLocation::None => None,
        };

        Ok(ExtractionResult {
            title: None,
            description,
            sections: result_sections,
            fields: Map::new(),
        })
    }
}

/// 节内段落按顺序拼接。
fn join_paragraphs(section: &RawSection) -> String {
    section
        .blocks
        .iter()
        .filter(|b| b.kind == BlockType::Paragraph)
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 单个节 → JSON 对象（name / paragraphs / items / subsections）。
fn section_to_json(raw: &RawSection) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("name".to_string(), Value::String(raw.title.clone()));

    let paragraphs: Vec<Value> = raw
        .blocks
        .iter()
        .filter(|b| b.kind == BlockType::Paragraph)
        .map(|b| Value::String(b.text.clone()))
        .collect();
    if !paragraphs.is_empty() {
        map.insert("paragraphs".to_string(), Value::Array(paragraphs));
    }

    let items: Vec<Value> = raw
        .blocks
        .iter()
        .filter(|b| b.kind == BlockType::Bullet)
        .map(|b| {
            let named = parse_named_item(&b.text);
            Value::Object(Map::from_iter([
                ("name".to_string(), Value::String(named.name)),
                ("detail".to_string(), Value::String(named.detail)),
            ]))
        })
        .collect();
    if !items.is_empty() {
        map.insert("items".to_string(), Value::Array(items));
    }

    if !raw.subsections.is_empty() {
        let children: Vec<Value> = raw
            .subsections
            .iter()
            .map(section_to_json)
            .map(Value::Object)
            .collect();
        map.insert("subsections".to_string(), Value::Array(children));
    }

    map
}

/// LLM 提取器：把规则说明 + Section 树发给 LLM，按规则填表提取。
///
/// LLM 客户端用 `quanttide-agent` 的 [`LLM`]。
pub struct LlmExtractor {
    pub llm: LLM,
}

impl LlmExtractor {
    pub fn new(llm: LLM) -> Self {
        Self { llm }
    }

    fn build_prompt(sections: &[RawSection], rule: &Artifact) -> String {
        let mut buf = String::new();
        buf.push_str(
            "你是文档解析器。按以下规则从文档中提取结构化数据，只输出 JSON，不要解释。\n\n",
        );
        buf.push_str("## 解析规则\n");
        buf.push_str(&rule.instruction());
        buf.push_str("\n## 文档内容\n");

        for section in sections {
            if !section.title.is_empty() {
                buf.push_str(&format!("\n### {}\n", section.title));
            }
            for block in &section.blocks {
                match block.kind {
                    BlockType::Paragraph => buf.push_str(&format!("{}\n", block.text)),
                    BlockType::Bullet => buf.push_str(&format!("- {}\n", block.text)),
                    BlockType::Separator => buf.push_str("---\n"),
                    BlockType::Heading => {
                        buf.push_str(&format!("{} {}\n", "#".repeat(block.level), block.text))
                    }
                }
            }
            for sub in &section.subsections {
                buf.push_str(&format!("\n#### {}\n", sub.title));
                for block in &sub.blocks {
                    match block.kind {
                        BlockType::Paragraph => buf.push_str(&format!("{}\n", block.text)),
                        BlockType::Bullet => buf.push_str(&format!("- {}\n", block.text)),
                        _ => {}
                    }
                }
            }
        }

        buf.push_str("\n## 输出格式\n");
        buf.push_str(
            "{\"title\": \"...\", \"description\": \"...\", \"sections\": [{\"name\": \"...\", \"paragraphs\": [...], \"items\": [{\"name\": \"...\", \"detail\": \"...\"}]}], \"fields\": {\"field\": \"...\"}}\n",
        );
        buf
    }

    /// 从 LLM 响应里取出 JSON 并落成提取结果；取不到 JSON 视为错误。
    pub fn parse_response(response: &str) -> Result<ExtractionResult, Error> {
        let value = quanttide_agent::parse_structured_output(response).map_err(Error::Parse)?;
        let title = optional_string(&value, "title")?;
        let description = optional_string(&value, "description")?;
        let sections = match value.get("sections") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(items)) => {
                let mut parsed = Vec::with_capacity(items.len());
                for item in items {
                    match item.as_object() {
                        Some(map) => parsed.push(map.clone()),
                        None => return Err(Error::Parse("sections 元素不是对象".to_string())),
                    }
                }
                parsed
            }
            Some(_) => return Err(Error::Parse("sections 不是数组".to_string())),
        };
        let fields = match value.get("fields") {
            None | Some(Value::Null) => Map::new(),
            Some(Value::Object(map)) => map.clone(),
            Some(_) => return Err(Error::Parse("fields 不是对象".to_string())),
        };
        Ok(ExtractionResult {
            title,
            description,
            sections,
            fields,
        })
    }
}

fn optional_string(value: &Value, key: &str) -> Result<Option<String>, Error> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(Error::Parse(format!("{key} 不是字符串"))),
    }
}

/// 必填字符串：判断结果缺 decision / reason 视为错误，不让「缺失」看起来正常。
fn required_string(value: &Value, key: &str) -> Result<String, Error> {
    optional_string(value, key)?.ok_or_else(|| Error::Parse(format!("判断结果缺少 {key}")))
}

/// 从 workflow 的 rules 文本里读关键词（含「关键词」的行，冒号后按分隔符切）。
///
/// 关键词是数据：locate 这类 workflow 把关键词表写在 YAML 里，代码不持有关键词表。
fn keywords_from_rules(rules: &str) -> Vec<String> {
    let mut keywords = Vec::new();
    for line in rules.lines() {
        let Some((label, rest)) = line.split_once(['：', ':']) else {
            continue;
        };
        if !label.contains("关键词") {
            continue;
        }
        for token in rest.split(['、', '，', ',', '；', ';', '。', '\n', ' ', '\t']) {
            let token = token.trim();
            if !token.is_empty() {
                keywords.push(token.to_string());
            }
        }
    }
    keywords
}

impl SemanticExtractor for LlmExtractor {
    fn extract(&self, sections: &[RawSection], rule: &Artifact) -> Result<ExtractionResult, Error> {
        let prompt = Self::build_prompt(sections, rule);
        let response = self
            .llm
            .complete(&[Message::new("user", &prompt)], CompleteOptions::default())?;
        Self::parse_response(&response.content)
    }
}

// ---------------------------------------------------------------------------
// 计算引擎：三个动词
// ---------------------------------------------------------------------------

/// 条目参与关键词匹配与展示时的文本：字符串取本体，其余走 JSON。
fn item_text(item: &Value) -> String {
    match item {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// 计算结果。
#[derive(Debug, Clone, Default)]
pub struct WorkflowResult {
    pub items: Vec<Value>,
    pub judgments: Vec<Judgment>,
    /// scan 步定位到的候选——scan 定位候选，不改动条目。
    pub candidates: Vec<Candidate>,
}

/// 判断结果：decision 是判定值，reason 是判定理由（判断必须给出理由，不设缺失）。
#[derive(Debug, Clone)]
pub struct Judgment {
    pub decision: String,
    pub reason: String,
}

/// 候选条目。
#[derive(Debug, Clone)]
pub struct Candidate {
    pub source: String,
    pub text: String,
}

/// 计算引擎：scan / judge / merge 三个动词 + workflow 编排。
pub struct Engine {
    pub llm: LLM,
}

impl Engine {
    pub fn new(llm: LLM) -> Self {
        Self { llm }
    }

    /// 编排：按 workflow 声明依次执行 steps。
    pub fn run(&self, workflow: &Workflow, input: Vec<Value>) -> Result<WorkflowResult, Error> {
        let mut items = input;
        let mut judgments: Vec<Judgment> = Vec::new();
        let mut candidates: Vec<Candidate> = Vec::new();

        for step in &workflow.steps {
            match step.verb {
                // 关键词是数据，从 workflow 的 rules 里读，不在代码里；scan 定位候选，不动条目
                Verb::Scan => {
                    let keywords = keywords_from_rules(&workflow.rules);
                    candidates.extend(Self::scan(&items, &keywords));
                }
                Verb::Judge => {
                    let judgment = self.judge(&workflow.rules, &items)?;
                    judgments.push(judgment);
                    // judge 产出去向标注，不改 items
                }
                Verb::Merge => {
                    items = Self::merge(&items, &[]);
                }
            }
        }

        Ok(WorkflowResult {
            items,
            judgments,
            candidates,
        })
    }

    /// 按关键词定位候选（代码）。
    pub fn scan(items: &[Value], keywords: &[String]) -> Vec<Candidate> {
        let mut candidates = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let text = item_text(item);
            if keywords.iter().any(|kw| text.contains(kw.as_str())) {
                candidates.push(Candidate {
                    source: index.to_string(),
                    text,
                });
            }
        }
        candidates
    }

    /// 让 LLM 按判据判断（LLM）：结构化进、结构化出，decision 与 reason 都必填。
    pub fn judge(&self, criteria: &str, items: &[Value]) -> Result<Judgment, Error> {
        let listed = Value::Array(items.to_vec());
        let prompt = format!(
            "按以下判据判断，只输出 JSON，不要解释。\n\n## 判据\n{criteria}\n\n## 条目\n{listed}\n\n## 输出格式\n{{\"decision\": \"...\", \"reason\": \"...\"}}\n"
        );
        let response = self
            .llm
            .complete(&[Message::new("user", &prompt)], CompleteOptions::default())?;
        let value =
            quanttide_agent::parse_structured_output(&response.content).map_err(Error::Parse)?;
        let decision = required_string(&value, "decision")?;
        let reason = required_string(&value, "reason")?;
        Ok(Judgment { decision, reason })
    }

    /// 语义填表（LLM 首选）：按 Artifact 规则从 Section 树提取结构化数据。
    pub fn extract(
        &self,
        sections: &[RawSection],
        rule: &Artifact,
    ) -> Result<ExtractionResult, Error> {
        let prompt = LlmExtractor::build_prompt(sections, rule);
        let response = self
            .llm
            .complete(&[Message::new("user", &prompt)], CompleteOptions::default())?;
        LlmExtractor::parse_response(&response.content)
    }

    /// 按策略合并条目（代码）。
    pub fn merge(existing: &[Value], incoming: &[Value]) -> Vec<Value> {
        let mut merged = existing.to_vec();
        merged.extend_from_slice(incoming);
        merged
    }
}
