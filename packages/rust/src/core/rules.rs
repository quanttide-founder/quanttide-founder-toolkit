//! core/rules：Artifact（名词定义）与 Workflow（动词定义）的 YAML 加载。
//!
//! Artifact 定义一个语义模型长什么样、从哪提取，外加「什么时候属于它」的判据——
//! 去向分类的类别就是一组 Artifact（`name` 是 key），判据由 LLM 执行，不由代码判。
//! Workflow 定义一个任务怎么做，用引擎的三个动词（scan/judge/merge）组装。

use std::fmt;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Deserializer, de};

use crate::error::Error;

// ---------------------------------------------------------------------------
// Artifact：名词定义
// ---------------------------------------------------------------------------

/// 一份文档的读法 + 归类判据：标题从哪来、说明在哪、章节怎么切、提取哪些字段、什么时候属于它。
#[derive(Debug, Clone, Deserialize)]
pub struct Artifact {
    /// 类别 key（如 `insight`）——去向分类输出的就是这个名字；空串表示不参与分类。
    #[serde(default)]
    pub name: String,
    pub document_type: String,
    /// 判据：它是什么、怎么判断、边界在哪（正例 / 反例 / 易混例）。
    #[serde(default)]
    pub criteria: Option<Criteria>,
    pub title: TitleRule,
    pub description: DescriptionRule,
    pub sections: SectionsRule,
    /// 分级类别：category 是数据，节标题按 `titles` 对号，分级名取 `name`。
    #[serde(default)]
    pub grades: Vec<GradeRule>,
}

/// 判据：意图 + 判据 + 边界（正例、反例、易混例，易混例带裁决）。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Criteria {
    #[serde(default)]
    pub intent: String,
    #[serde(default)]
    pub rules: String,
    #[serde(default)]
    pub positive: Vec<String>,
    #[serde(default)]
    pub negative: Vec<String>,
    #[serde(default)]
    pub ambiguous: Vec<String>,
}

impl Criteria {
    /// 判据的自然语言描述，供 LLM 执行判断。
    pub fn instruction(&self) -> String {
        let mut buf = String::new();
        let _ = writeln!(buf, "意图: {}", self.intent);
        let _ = write!(buf, "判据: {}", self.rules.trim());
        for (label, examples) in [("正例", &self.positive), ("反例", &self.negative)] {
            if !examples.is_empty() {
                let _ = write!(buf, "\n{label}: {}", examples.join("；"));
            }
        }
        if !self.ambiguous.is_empty() {
            let _ = write!(buf, "\n易混例（含裁决）: {}", self.ambiguous.join("；"));
        }
        buf
    }
}

/// 一个分级类别。
#[derive(Debug, Clone, Deserialize)]
pub struct GradeRule {
    /// 分级名（如 `confirmed`），判断结果与文档解析都用它。
    pub name: String,
    /// 对应的节标题（如「已确认」）。
    pub titles: Vec<String>,
    #[serde(default)]
    pub note: String,
}

/// 标题来源：固定取值，加载期校验（`split_by: h7` 这类写法加载即失败）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TitleSource {
    /// 第一个 H1 标题。
    FirstH1,
    /// 文件名去掉 `.md`。
    Filename,
    /// 不提取。
    None,
}

impl fmt::Display for TitleSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TitleSource::FirstH1 => write!(f, "第一个 H1 标题"),
            TitleSource::Filename => write!(f, "文件名去掉 .md"),
            TitleSource::None => write!(f, "不提取"),
        }
    }
}

/// 标题规则。
#[derive(Debug, Clone, Deserialize)]
pub struct TitleRule {
    pub source: TitleSource,
    pub fallback: TitleSource,
}

/// 说明位置：固定取值，规则降级的提取路径由它配置，不由代码写死。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DescriptionLocation {
    /// H1 之后、首个 H2 之前的段落。
    Preface,
    /// 首个正式节的段落。
    Section,
    /// 不提取。
    None,
}

impl fmt::Display for DescriptionLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DescriptionLocation::Preface => write!(f, "H1 之后、首个 H2 之前的段落"),
            DescriptionLocation::Section => write!(f, "首个正式节的段落"),
            DescriptionLocation::None => write!(f, "不提取"),
        }
    }
}

/// 说明规则。
#[derive(Debug, Clone, Deserialize)]
pub struct DescriptionRule {
    pub location: DescriptionLocation,
    pub meaning: String,
}

/// 章节切分层级：H1-H6，加载期校验。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum SplitLevel {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

impl SplitLevel {
    /// 标题层级（1-6），供 `split_sections` 用。
    pub const fn level(self) -> usize {
        match self {
            SplitLevel::H1 => 1,
            SplitLevel::H2 => 2,
            SplitLevel::H3 => 3,
            SplitLevel::H4 => 4,
            SplitLevel::H5 => 5,
            SplitLevel::H6 => 6,
        }
    }
}

impl fmt::Display for SplitLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// 章节切分与字段提取规则。
#[derive(Debug, Clone, Deserialize)]
pub struct SectionsRule {
    pub split_by: SplitLevel,
    #[serde(default)]
    pub subsections: Option<SplitLevel>,
    #[serde(default)]
    pub extract: Option<Vec<ExtractField>>,
    #[serde(default)]
    pub unknown_content: Option<String>,
}

/// 一个待提取字段。
#[derive(Debug, Clone, Deserialize)]
pub struct ExtractField {
    pub field: String,
    pub from: String,
    #[serde(default)]
    pub patterns: Option<Vec<String>>,
}

impl Artifact {
    /// 从 YAML 文本加载规则。
    pub fn from_yaml(content: &str) -> Result<Self, Error> {
        Ok(serde_yaml::from_str(content)?)
    }

    /// 从 YAML 文件加载规则。
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        Self::from_yaml(&fs::read_to_string(path)?)
    }

    /// 按本规则的切分层级分节（读法落地：`split_by` 不再是没人读的字段）。
    pub fn split(
        &self,
        blocks: &[crate::core::parse::Block],
    ) -> Vec<crate::core::parse::RawSection> {
        crate::core::parse::split_sections(blocks, self.sections.split_by.level())
    }

    /// 判据的自然语言描述，供 LLM 执行归类判断；没有判据则不参与分类。
    pub fn criteria_instruction(&self) -> Option<String> {
        self.criteria.as_ref().map(Criteria::instruction)
    }

    /// 规则的自然语言描述，供 LLM 理解「怎么读」。
    pub fn instruction(&self) -> String {
        let mut buf = String::new();
        let _ = writeln!(buf, "文档类型: {}", self.document_type);
        let _ = writeln!(
            buf,
            "标题: {}（兜底: {}）",
            self.title.source, self.title.fallback
        );
        let _ = writeln!(
            buf,
            "说明: {}，含义是{}",
            self.description.location, self.description.meaning
        );
        match self.sections.subsections {
            Some(subsections) => {
                let _ = writeln!(
                    buf,
                    "章节切分: 按 {} 切分，{} 为子节",
                    self.sections.split_by, subsections
                );
            }
            None => {
                let _ = writeln!(buf, "章节切分: 按 {} 切分", self.sections.split_by);
            }
        }

        if let Some(extract) = &self.sections.extract {
            let _ = writeln!(buf, "提取字段:");
            for field in extract {
                let _ = writeln!(buf, "  - {}: {}", field.field, field.from);
                if let Some(patterns) = &field.patterns {
                    let _ = writeln!(buf, "    格式: {}", patterns.join(" / "));
                }
            }
        }

        if let Some(unknown) = &self.sections.unknown_content {
            let _ = writeln!(buf, "未知内容: {unknown}");
        }

        buf
    }
}

// ---------------------------------------------------------------------------
// Workflow：动词定义
// ---------------------------------------------------------------------------

/// Workflow：一个任务怎么做，由 steps 组装。
#[derive(Debug, Clone, Deserialize)]
pub struct Workflow {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub input: String,
    #[serde(default)]
    pub output: Vec<String>,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub rules: String,
    /// 判断类任务的可选决策（如合并策略），名字在 YAML 里，不在代码里。
    #[serde(default)]
    pub actions: Vec<Action>,
}

impl Workflow {
    /// 从 YAML 文本加载工作流。
    pub fn from_yaml(content: &str) -> Result<Self, Error> {
        Ok(serde_yaml::from_str(content)?)
    }

    /// 从 YAML 文件加载工作流。
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        Self::from_yaml(&fs::read_to_string(path)?)
    }
}

/// 引擎认识的三个动词：编译期取值，加载期校验。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Scan,
    Judge,
    Merge,
}

impl Verb {
    pub const fn as_str(self) -> &'static str {
        match self {
            Verb::Scan => "scan",
            Verb::Judge => "judge",
            Verb::Merge => "merge",
        }
    }

    /// 从 YAML 里的动词名解析；不认识的动词在这里就失败，不留到运行期。
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "scan" => Some(Verb::Scan),
            "judge" => Some(Verb::Judge),
            "merge" => Some(Verb::Merge),
            _ => None,
        }
    }
}

impl fmt::Display for Verb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Verb {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Verb::parse(&name).ok_or_else(|| {
            de::Error::custom(format!("未知动词: {name}（只认 scan / judge / merge）"))
        })
    }
}

/// Step：workflow 的一个步骤，verb 只能是 scan / judge / merge。
///
/// YAML 里写成 `- scan: 描述`——一个单键映射，键是动词、值是步骤描述。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub verb: Verb,
    pub description: String,
}

impl<'de> Deserialize<'de> for Step {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = <std::collections::HashMap<String, String>>::deserialize(deserializer)?;
        let (verb, description) = map
            .drain()
            .next()
            .ok_or_else(|| de::Error::custom("步骤不能为空"))?;
        let verb = Verb::parse(&verb).ok_or_else(|| {
            de::Error::custom(format!("未知动词: {verb}（只认 scan / judge / merge）"))
        })?;
        Ok(Step { verb, description })
    }
}

/// Action：判断类任务的一个可选决策，YAML 里同样写成单键映射（`- crossLink: 描述`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    pub name: String,
    pub description: String,
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = <std::collections::HashMap<String, String>>::deserialize(deserializer)?;
        let (name, description) = map
            .drain()
            .next()
            .ok_or_else(|| de::Error::custom("决策不能为空"))?;
        Ok(Action { name, description })
    }
}
