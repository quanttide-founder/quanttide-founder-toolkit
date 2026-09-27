//! memory/models：域模型（纯数据）。
//!
//! `parse` 只吃字符串——文件读取归 repository，本文件不碰磁盘。

use std::sync::LazyLock;

use chrono::NaiveDate;
use regex::Regex;

use crate::core::parse::{
    BlockType, MarkdownDocument, NamedItem, RawSection, TextSection, parse_named_item,
    split_sections,
};

/// 日志分段：`---` 类分隔线。
static SEGMENT_SPLIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^[ \t]*-{2,}[ \t]*$").unwrap());

// ---------------------------------------------------------------------------
// 时间线：日志
// ---------------------------------------------------------------------------

/// 日志来源：集根的当天日志，还是 `journal/` 归档。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalSource {
    Root,
    Archive,
}

/// 一篇日志：文件名即日期。
#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub date: NaiveDate,
    pub path: String,
    pub source: JournalSource,
    pub content: String,
}

impl JournalEntry {
    /// 按 `---` 类分隔线切出的会话段。
    pub fn segments(&self) -> Vec<&str> {
        SEGMENT_SPLIT
            .split(&self.content)
            .map(str::trim)
            .filter(|segment| !segment.is_empty())
            .collect()
    }

    /// 去掉全部空白后的字数。
    pub fn character_count(&self) -> usize {
        self.content.chars().filter(|c| !c.is_whitespace()).count()
    }
}

// ---------------------------------------------------------------------------
// 特征层：个人档案
// ---------------------------------------------------------------------------

/// 一份档案：H1 标题 + 说明 + H2 主题节。
#[derive(Debug, Clone)]
pub struct ProfileDoc {
    pub path: String,
    pub name: String,
    /// 一级标题，缺失时由文件名兜底。
    pub title: Option<String>,
    /// 标题与首个二级标题之间的来源说明。
    pub description: String,
    pub sections: Vec<TextSection>,
}

impl ProfileDoc {
    pub fn parse(path: impl Into<String>, content: &str) -> Self {
        let path = path.into();
        let name = base_name(&path);
        let doc = MarkdownDocument::parse(path.clone(), content);
        let raw = split_sections(&doc.blocks, 2);
        let description = raw
            .first()
            .map(|preface| {
                preface
                    .blocks
                    .iter()
                    .filter(|b| b.kind == BlockType::Paragraph)
                    .map(|b| b.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        ProfileDoc {
            path,
            name,
            title: doc.title().map(str::to_string),
            description,
            sections: raw.iter().skip(1).map(TextSection::from_raw).collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// 认知层：洞察
// ---------------------------------------------------------------------------

/// 证据分级：已确认 / 假说。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsightGrade {
    Confirmed,
    Hypothesis,
}

/// 一条洞察：命题 + 解释 + 证据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsightItem {
    /// 命题。
    pub statement: String,
    /// 解释。
    pub detail: String,
    /// 「依据：」之后的证据。
    pub evidence: Option<String>,
}

impl InsightItem {
    pub fn new(statement: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            statement: statement.into(),
            detail: detail.into(),
            evidence: None,
        }
    }
}

/// 洞察文档的一节：标题 + 分级 + 条目/段落。
#[derive(Debug, Clone)]
pub struct InsightSection {
    pub title: String,
    /// 已确认/假说分级；主题式节为 None。
    pub grade: Option<InsightGrade>,
    pub items: Vec<InsightItem>,
    pub paragraphs: Vec<String>,
}

/// 一份洞察文档。
#[derive(Debug, Clone)]
pub struct InsightDoc {
    pub path: String,
    pub name: String,
    pub title: Option<String>,
    pub sections: Vec<InsightSection>,
}

impl InsightDoc {
    /// 某一分级下的全部条目。
    pub fn items_of(&self, grade: InsightGrade) -> impl Iterator<Item = &InsightItem> {
        self.sections
            .iter()
            .filter(move |s| s.grade == Some(grade))
            .flat_map(|s| s.items.iter())
    }

    pub fn parse(path: impl Into<String>, content: &str) -> Self {
        let path = path.into();
        let name = base_name(&path);
        let doc = MarkdownDocument::parse(path.clone(), content);
        let raw = split_sections(&doc.blocks, 2);
        let mut parsed: Vec<InsightSection> = Vec::new();

        if raw.len() == 1 {
            // 无二级标题：全文作为一节（主题式散文）。
            let mut built = build_section(&raw[0]);
            if built.title.is_empty() {
                built.title = doc.title().map_or_else(|| name.clone(), str::to_string);
            }
            parsed.push(built);
        } else {
            parsed.extend(raw.iter().skip(1).map(build_section));
        }

        InsightDoc {
            path,
            name,
            title: doc.title().map(str::to_string),
            sections: parsed,
        }
    }
}

fn build_section(raw: &RawSection) -> InsightSection {
    let grade = if raw.title.contains("已确认") {
        Some(InsightGrade::Confirmed)
    } else if raw.title.contains("假说") {
        Some(InsightGrade::Hypothesis)
    } else {
        None
    };
    let mut section = InsightSection {
        title: raw.title.clone(),
        grade,
        items: Vec::new(),
        paragraphs: Vec::new(),
    };
    for block in &raw.blocks {
        match block.kind {
            BlockType::Bullet => {
                section.items.push(parse_insight_item(&block.text));
            }
            BlockType::Paragraph => {
                section.paragraphs.push(block.text.clone());
            }
            _ => {}
        }
    }
    section
}

/// 条目文本 → 命题 / 解释 / 证据（「依据：」之后）。
fn parse_insight_item(text: &str) -> InsightItem {
    let named = parse_named_item(text);
    let mut detail = named.detail;
    let mut evidence = None;
    if let Some(marker) = detail.find("依据：") {
        evidence = Some(detail[marker + "依据：".len()..].trim().to_string());
        detail = detail[..marker].trim().to_string();
    }
    InsightItem {
        statement: named.name,
        detail,
        evidence,
    }
}

// ---------------------------------------------------------------------------
// 方向层：路线图
// ---------------------------------------------------------------------------

/// 一份路线图：目标 + 元目标/核心问题/已决策/待决策 + 主题节。
#[derive(Debug, Clone)]
pub struct RoadmapDoc {
    pub path: String,
    pub name: String,
    pub title: Option<String>,
    /// 「## 目标」的正文。
    pub goal: Option<String>,
    /// 「### 元目标」。
    pub meta_goals: Vec<NamedItem>,
    pub core_problems: Vec<NamedItem>,
    pub decided: Vec<NamedItem>,
    pub pending: Vec<NamedItem>,
    /// 非方向层结构的主题节（写作集路线图为主题式）。
    pub theme_sections: Vec<TextSection>,
}

impl RoadmapDoc {
    pub fn parse(path: impl Into<String>, content: &str) -> Self {
        let path = path.into();
        let name = base_name(&path);
        let doc = MarkdownDocument::parse(path.clone(), content);
        let raw = split_sections(&doc.blocks, 2);

        let mut goal = None;
        let mut meta_goals: Vec<NamedItem> = Vec::new();
        let mut core_problems: Vec<NamedItem> = Vec::new();
        let mut decided: Vec<NamedItem> = Vec::new();
        let mut pending: Vec<NamedItem> = Vec::new();
        let mut theme_sections: Vec<TextSection> = Vec::new();

        for section in raw.iter().skip(1) {
            let paragraphs = section
                .blocks
                .iter()
                .filter(|b| b.kind == BlockType::Paragraph)
                .map(|b| b.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            match section.title.as_str() {
                "目标" => {
                    goal = if paragraphs.is_empty() {
                        None
                    } else {
                        Some(paragraphs)
                    };
                    meta_goals = section
                        .subsections
                        .iter()
                        .filter(|s| s.title == "元目标")
                        .flat_map(|s| s.blocks.iter())
                        .filter(|b| b.kind == BlockType::Bullet)
                        .map(|b| parse_named_item(&b.text))
                        .collect();
                }
                "核心问题" => core_problems = named_bullets(section),
                "已决策" => decided = named_bullets(section),
                "待决策" => pending = named_bullets(section),
                _ => theme_sections.push(TextSection::from_raw(section)),
            }
        }

        RoadmapDoc {
            path,
            name,
            title: doc.title().map(str::to_string),
            goal,
            meta_goals,
            core_problems,
            decided,
            pending,
            theme_sections,
        }
    }
}

fn named_bullets(section: &RawSection) -> Vec<NamedItem> {
    section
        .blocks
        .iter()
        .filter(|b| b.kind == BlockType::Bullet)
        .map(|b| parse_named_item(&b.text))
        .collect()
}

// ---------------------------------------------------------------------------
// 内部工具
// ---------------------------------------------------------------------------

/// 文件名去掉 `.md`。
fn base_name(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(".md").unwrap_or(name).to_string()
}
