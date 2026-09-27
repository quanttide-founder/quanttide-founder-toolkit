//! fiction/models：域模型（纯数据）。
//!
//! `discover` 只认目录/文件名约定，读文件由 repository 与 discover 内部完成。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::error::Error;

// ---------------------------------------------------------------------------
// 创作阶段
// ---------------------------------------------------------------------------

/// 阶段目录名：`{N}_{名称}`。
static STAGE_DIR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d+)_(.+)$").unwrap());

/// 创作阶段目录：`{N}_{名称}` 模式动态识别，不硬编码阶段名。
///
/// 各小说阶段名不同（职场=灵感/场景/初稿/改稿/定稿，校园=素材/提纲/初稿/改稿），
/// 按编号前缀发现阶段，编号即流程顺序。
#[derive(Debug, Clone)]
pub struct Stage {
    /// 阶段编号（1-5），来自目录名前缀。
    pub number: u32,
    /// 阶段名称（灵感/场景/初稿/改稿/定稿…），来自目录名后缀。
    pub name: String,
    pub directory: PathBuf,
    pub chapters: Vec<Chapter>,
}

impl Stage {
    /// 目录名是否是阶段目录。
    pub fn is_stage_dir_name(name: &str) -> bool {
        STAGE_DIR.is_match(name)
    }

    /// 从目录条目发现阶段。返回按编号排序的阶段列表。
    pub fn discover(novel_root: impl AsRef<Path>) -> Result<Vec<Stage>, Error> {
        let mut stages: Vec<Stage> = Vec::new();
        for entry in sorted_entries(novel_root.as_ref())? {
            if !entry.is_dir() {
                continue;
            }
            let Some(name) = file_name(&entry) else {
                continue;
            };
            let Some(caps) = STAGE_DIR.captures(&name) else {
                continue;
            };
            let number = caps[1]
                .parse::<u32>()
                .map_err(|_| Error::Parse(format!("阶段编号无效: {name}")))?;
            stages.push(Stage {
                number,
                name: caps[2].to_string(),
                directory: entry.clone(),
                chapters: Chapter::discover(&entry)?,
            });
        }
        stages.sort_by_key(|stage| stage.number);
        Ok(stages)
    }
}

// ---------------------------------------------------------------------------
// 章节
// ---------------------------------------------------------------------------

/// 章节文件：`{序号}_{标题}.md`。
///
/// 编号语义：
/// - 序号对应正文最终阅读顺序，各阶段共用同一编号轴
/// - 预留空号（宁空勿移），同一序号可有多份（初稿/改稿/定稿）
/// - 未编号文件（如「地摊火锅.md」）是替代草稿，不占编号
/// - `0_` 前缀是非正文文件（前言等），不占正文章节号
#[derive(Debug, Clone)]
pub struct Chapter {
    /// 章节序号，None 表示未编号（替代草稿）。
    pub number: Option<u32>,
    /// 标题（文件名去掉序号前缀与扩展名）。
    pub title: String,
    pub file: PathBuf,
    pub content: String,
}

impl Chapter {
    /// 前言等非正文。
    pub fn is_preface(&self) -> bool {
        self.number == Some(0)
    }

    /// 未编号的替代草稿。
    pub fn is_unnumbered(&self) -> bool {
        self.number.is_none()
    }

    /// 从阶段目录发现章节。跳过 README。
    pub fn discover(stage_dir: impl AsRef<Path>) -> Result<Vec<Chapter>, Error> {
        let mut chapters: Vec<Chapter> = Vec::new();
        for entry in sorted_entries(stage_dir.as_ref())? {
            if !entry.is_file() {
                continue;
            }
            let Some(name) = file_name(&entry) else {
                continue;
            };
            if name == "README.md" || !name.ends_with(".md") {
                continue;
            }
            let Some(caps) = CHAPTER_FILE.captures(&name) else {
                continue;
            };
            let number = match caps.get(1) {
                Some(digits) => Some(
                    digits
                        .as_str()
                        .parse::<u32>()
                        .map_err(|_| Error::Parse(format!("章节编号无效: {name}")))?,
                ),
                None => None,
            };
            chapters.push(Chapter {
                number,
                title: caps[2].to_string(),
                file: entry.clone(),
                content: fs::read_to_string(&entry)?,
            });
        }
        chapters.sort_by(|a, b| {
            (a.number.unwrap_or(u32::MAX), &a.file).cmp(&(b.number.unwrap_or(u32::MAX), &b.file))
        });
        Ok(chapters)
    }
}

static CHAPTER_FILE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:(\d+)_)?(.+)\.md$").unwrap());

// ---------------------------------------------------------------------------
// 小说
// ---------------------------------------------------------------------------

/// 小说：一个创作目录，含 index.md（晋江资料）与若干创作阶段。
#[derive(Debug, Clone)]
pub struct Novel {
    /// 目录名（职场言情/校园言情/重生言情）。
    pub name: String,
    pub directory: PathBuf,
    pub stages: Vec<Stage>,
    /// index.md 原文，语义提取由 Artifact 规则 + LLM 完成。
    pub index_content: Option<String>,
}

impl Novel {
    /// 全部章节（跨阶段），按编号排序。
    pub fn all_chapters(&self) -> Vec<&Chapter> {
        let mut chapters: Vec<&Chapter> =
            self.stages.iter().flat_map(|s| s.chapters.iter()).collect();
        chapters.sort_by(|a, b| {
            (a.number.unwrap_or(u32::MAX), &a.file).cmp(&(b.number.unwrap_or(u32::MAX), &b.file))
        });
        chapters
    }

    /// 按阶段名查找。
    pub fn stage_by_name(&self, name: &str) -> Option<&Stage> {
        self.stages.iter().find(|stage| stage.name == name)
    }
}

// ---------------------------------------------------------------------------
// 观察站
// ---------------------------------------------------------------------------

/// 情绪日记条目。
#[derive(Debug, Clone)]
pub struct EmotionalDiary {
    pub title: String,
    pub file: PathBuf,
    pub content: String,
}

/// 社会观察条目。
#[derive(Debug, Clone)]
pub struct SocialObservation {
    pub title: String,
    pub file: PathBuf,
    pub content: String,
}

/// 观察站：跨系列共用的观察素材（情绪日记 + 社会观察）。
///
/// 情绪日记是母题的发现来源；创作日志/创作谈/创作设定已迁至 memory 仓库的 write/ 记忆集。
#[derive(Debug, Clone, Default)]
pub struct Observation {
    pub emotional_diaries: Vec<EmotionalDiary>,
    pub social_observations: Vec<SocialObservation>,
}

// ---------------------------------------------------------------------------
// 内部工具
// ---------------------------------------------------------------------------

/// 目录条目按路径排序。
fn sorted_entries(dir: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<_, _>>()?;
    entries.sort();
    Ok(entries)
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}
