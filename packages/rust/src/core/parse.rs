//! core/parse：Markdown 解析 + 结构切分。
//!
//! 文本 → 块序列 → Section 树。通用，不认业务。

use std::sync::LazyLock;

use regex::Regex;

// ---------------------------------------------------------------------------
// 通用 Markdown 块解析
// ---------------------------------------------------------------------------

/// 块类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Heading,
    Bullet,
    Paragraph,
    Separator,
}

/// 一个 Markdown 块。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub kind: BlockType,
    /// 仅 heading 有效：1-6。
    pub level: usize,
    /// heading 去掉 `#` 后的文本；bullet 去掉标记后的文本；paragraph 原文。
    pub text: String,
}

impl Block {
    pub fn heading(level: usize, text: impl Into<String>) -> Self {
        Self {
            kind: BlockType::Heading,
            level,
            text: text.into(),
        }
    }

    pub fn bullet(text: impl Into<String>) -> Self {
        Self {
            kind: BlockType::Bullet,
            level: 0,
            text: text.into(),
        }
    }

    pub fn paragraph(text: impl Into<String>) -> Self {
        Self {
            kind: BlockType::Paragraph,
            level: 0,
            text: text.into(),
        }
    }

    pub fn separator() -> Self {
        Self {
            kind: BlockType::Separator,
            level: 0,
            text: String::new(),
        }
    }
}

static HEADING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(#{1,6})\s+(.*?)\s*#*\s*$").unwrap());
static BULLET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:[-*]|\d+\.)\s+(.*)$").unwrap());
static SEPARATOR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-{2,}\s*$").unwrap());

/// 解析后的 Markdown 文档。
#[derive(Debug, Clone)]
pub struct MarkdownDocument {
    pub path: String,
    pub blocks: Vec<Block>,
}

impl MarkdownDocument {
    pub fn parse(path: impl Into<String>, content: &str) -> Self {
        let mut blocks: Vec<Block> = Vec::new();
        let mut para = String::new();

        for raw in content.split('\n') {
            let raw = raw.strip_suffix('\r').unwrap_or(raw);
            let line = raw.trim();
            if line.is_empty() {
                flush(&mut blocks, &mut para);
                continue;
            }
            if let Some(caps) = HEADING.captures(line) {
                flush(&mut blocks, &mut para);
                blocks.push(Block::heading(caps[1].len(), &caps[2]));
                continue;
            }
            if SEPARATOR.is_match(line) {
                flush(&mut blocks, &mut para);
                blocks.push(Block::separator());
                continue;
            }
            if let Some(caps) = BULLET.captures(line) {
                flush(&mut blocks, &mut para);
                blocks.push(Block::bullet(&caps[1]));
                continue;
            }
            if para.is_empty() {
                para.push_str(line);
            } else {
                para.push('\n');
                para.push_str(line);
            }
        }
        flush(&mut blocks, &mut para);

        Self {
            path: path.into(),
            blocks,
        }
    }

    /// 首个一级标题，无则返回 None。
    pub fn title(&self) -> Option<&str> {
        self.blocks
            .iter()
            .find(|b| b.kind == BlockType::Heading && b.level == 1)
            .map(|b| b.text.as_str())
    }
}

fn flush(blocks: &mut Vec<Block>, para: &mut String) {
    let text = para.trim();
    if !text.is_empty() {
        blocks.push(Block::paragraph(text));
    }
    para.clear();
}

// ---------------------------------------------------------------------------
// 分节：按指定层级的标题切分，下一层级标题成为子节
// ---------------------------------------------------------------------------

/// 一个节及其子节。
#[derive(Debug, Clone)]
pub struct RawSection {
    pub level: usize,
    pub title: String,
    pub blocks: Vec<Block>,
    pub subsections: Vec<RawSection>,
}

impl RawSection {
    fn new(level: usize, title: impl Into<String>) -> Self {
        Self {
            level,
            title: title.into(),
            blocks: Vec::new(),
            subsections: Vec::new(),
        }
    }
}

/// 以 `level` 级标题为边界切分；首个元素是首节之前的前言（title 为空串）。
pub fn split_sections(blocks: &[Block], level: usize) -> Vec<RawSection> {
    let mut sections = vec![RawSection::new(level, "")];
    let mut current = 0usize;
    let mut subsection: Option<usize> = None;

    for block in blocks {
        if block.kind == BlockType::Heading && block.level == level {
            sections.push(RawSection::new(level, block.text.clone()));
            current = sections.len() - 1;
            subsection = None;
        } else if block.kind == BlockType::Heading && block.level == level + 1 {
            let target = &mut sections[current];
            target
                .subsections
                .push(RawSection::new(level + 1, block.text.clone()));
            subsection = Some(target.subsections.len() - 1);
        } else {
            match subsection {
                Some(index) => sections[current].subsections[index]
                    .blocks
                    .push(block.clone()),
                None => sections[current].blocks.push(block.clone()),
            }
        }
    }

    sections
}

/// 通用节：主题段落 + 条目 + 子节，供档案与路线图主题节复用。
#[derive(Debug, Clone, Default)]
pub struct TextSection {
    pub title: String,
    pub paragraphs: Vec<String>,
    pub bullets: Vec<String>,
    pub subsections: Vec<TextSection>,
}

impl TextSection {
    pub fn from_raw(raw: &RawSection) -> Self {
        let mut section = TextSection {
            title: raw.title.clone(),
            ..Default::default()
        };
        for block in &raw.blocks {
            match block.kind {
                BlockType::Paragraph => section.paragraphs.push(block.text.clone()),
                BlockType::Bullet => section.bullets.push(block.text.clone()),
                _ => {}
            }
        }
        for child in &raw.subsections {
            section.subsections.push(TextSection::from_raw(child));
        }
        section
    }
}

// ---------------------------------------------------------------------------
// 条目解析：`- **名称**：详情`
// ---------------------------------------------------------------------------

static NAMED_BOLD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\*\*(.+?)\*\*\s*[：:]\s*(.*)$").unwrap());

/// 把 `- **名称**：详情` 解析为结构化条目；无加粗时以首个全角/半角冒号切分。
pub fn parse_named_item(text: &str) -> NamedItem {
    if let Some(caps) = NAMED_BOLD.captures(text) {
        return NamedItem {
            name: caps[1].to_string(),
            detail: caps[2].to_string(),
        };
    }

    let fullwidth = text.find('：');
    let halfwidth = text.find(':');
    let index = match (fullwidth, halfwidth) {
        (Some(f), None) => Some(f),
        (Some(f), Some(h)) if f < h => Some(f),
        (_, Some(h)) => Some(h),
        (None, None) => None,
    };
    match index {
        Some(index) if index > 0 => {
            let colon = text[index..].chars().next().unwrap_or('：');
            NamedItem {
                name: text[..index].trim().to_string(),
                detail: text[index + colon.len_utf8()..].trim().to_string(),
            }
        }
        _ => NamedItem {
            name: text.trim().to_string(),
            detail: String::new(),
        },
    }
}

/// 名称 + 详情的结构化条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedItem {
    pub name: String,
    pub detail: String,
}
