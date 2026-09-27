//! core/rules：Artifact（名词定义）与 Workflow（动词定义）的 YAML 加载。
//!
//! Artifact 定义一个语义模型长什么样、从哪提取。
//! Workflow 定义一个任务怎么做，用引擎的三个动词（scan/judge/merge）组装。

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::error::Error;

// ---------------------------------------------------------------------------
// Artifact：名词定义
// ---------------------------------------------------------------------------

/// 一份文档的读法：标题从哪来、说明在哪、章节怎么切、提取哪些字段。
#[derive(Debug, Clone, Deserialize)]
pub struct Artifact {
    pub document_type: String,
    pub title: TitleRule,
    pub description: DescriptionRule,
    pub sections: SectionsRule,
}

/// 标题规则。
#[derive(Debug, Clone, Deserialize)]
pub struct TitleRule {
    pub source: String,
    pub fallback: String,
}

/// 说明规则。
#[derive(Debug, Clone, Deserialize)]
pub struct DescriptionRule {
    pub location: String,
    pub meaning: String,
}

/// 章节切分与字段提取规则。
#[derive(Debug, Clone, Deserialize)]
pub struct SectionsRule {
    pub split_by: String,
    #[serde(default)]
    pub subsections: Option<String>,
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
        let _ = writeln!(
            buf,
            "章节切分: 按 {} 切分，{} 为子节",
            self.sections.split_by,
            self.sections.subsections.as_deref().unwrap_or("")
        );

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

/// Step：workflow 的一个步骤，verb 只能是 scan / judge / merge。
///
/// YAML 里写成 `- scan: 描述`——一个单键映射，键是动词、值是步骤描述。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub verb: String,
    pub description: String,
}

impl<'de> Deserialize<'de> for Step {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;
        let mut map = <std::collections::HashMap<String, String>>::deserialize(deserializer)?;
        match map.drain().next() {
            Some((verb, description)) => Ok(Step { verb, description }),
            None => Err(D::Error::custom("步骤不能为空")),
        }
    }
}
