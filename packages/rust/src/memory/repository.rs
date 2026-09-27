//! memory/repository：仓库装载——发现记忆集、逐层读文件。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use chrono::NaiveDate;
use regex::Regex;

use crate::error::Error;
use crate::memory::models::{InsightDoc, JournalEntry, JournalSource, ProfileDoc, RoadmapDoc};

/// 记忆集的层目录：含任一层的目录即一个记忆集。
const LAYER_DIRS: [&str; 4] = ["journal", "profile", "insight", "roadmap"];

/// 日志文件名：`YYYY-MM-DD.md`。
static DATE_FILE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{4})-(\d{2})-(\d{2})\.md$").unwrap());

/// 一个记忆集的装载结果。
#[derive(Debug, Clone)]
pub struct MemorySet {
    pub name: String,
    pub root: PathBuf,
    /// 按日期倒序。
    pub journals: Vec<JournalEntry>,
    pub profiles: Vec<ProfileDoc>,
    pub insights: Vec<InsightDoc>,
    pub roadmaps: Vec<RoadmapDoc>,
}

/// memory 仓库：根目录下含任一层目录的目录视为一个记忆集。
#[derive(Debug, Clone)]
pub struct MemoryRepository {
    pub root: PathBuf,
    pub sets: Vec<MemorySet>,
}

impl MemoryRepository {
    /// 装载仓库。
    pub fn load(root: impl AsRef<Path>) -> Result<Self, Error> {
        let root = root.as_ref();
        if !root.is_dir() {
            return Err(Error::RootNotFound {
                domain: "memory",
                path: root.display().to_string(),
            });
        }

        let mut sets = Vec::new();
        for entry in sorted_entries(root)? {
            if !entry.is_dir() {
                continue;
            }
            let Some(name) = file_name(&entry) else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            let is_set = LAYER_DIRS.iter().any(|layer| entry.join(layer).is_dir());
            if is_set {
                sets.push(parse_set(&entry, &name)?);
            }
        }
        Ok(MemoryRepository {
            root: root.to_path_buf(),
            sets,
        })
    }

    /// 全部记忆集的日志时间线，按日期倒序。
    pub fn all_journals(&self) -> Vec<&JournalEntry> {
        let mut journals: Vec<&JournalEntry> = self.sets.iter().flat_map(|s| &s.journals).collect();
        journals.sort_by_key(|entry| std::cmp::Reverse(entry.date));
        journals
    }
}

/// 装载一个记忆集的四层内容。
fn parse_set(set_root: &Path, name: &str) -> Result<MemorySet, Error> {
    let mut journals = Vec::new();
    read_journals(set_root, JournalSource::Root, &mut journals)?;
    read_journals(
        &set_root.join("journal"),
        JournalSource::Archive,
        &mut journals,
    )?;
    journals.sort_by_key(|entry| std::cmp::Reverse(entry.date));

    let mut profiles = Vec::new();
    for file in layer_files(set_root, "profile")? {
        profiles.push(ProfileDoc::parse(
            file.to_string_lossy(),
            &fs::read_to_string(&file)?,
        ));
    }
    let mut insights = Vec::new();
    for file in layer_files(set_root, "insight")? {
        insights.push(InsightDoc::parse(
            file.to_string_lossy(),
            &fs::read_to_string(&file)?,
        ));
    }
    let mut roadmaps = Vec::new();
    for file in layer_files(set_root, "roadmap")? {
        roadmaps.push(RoadmapDoc::parse(
            file.to_string_lossy(),
            &fs::read_to_string(&file)?,
        ));
    }

    Ok(MemorySet {
        name: name.to_string(),
        root: set_root.to_path_buf(),
        journals,
        profiles,
        insights,
        roadmaps,
    })
}

/// 读一个目录下的日志文件（只认 `YYYY-MM-DD.md`）。
fn read_journals(
    dir: &Path,
    source: JournalSource,
    out: &mut Vec<JournalEntry>,
) -> Result<(), Error> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in sorted_entries(dir)? {
        if !entry.is_file() {
            continue;
        }
        let Some(file_name) = file_name(&entry) else {
            continue;
        };
        let Some(caps) = DATE_FILE.captures(&file_name) else {
            continue;
        };
        let date_text = format!("{}-{}-{}", &caps[1], &caps[2], &caps[3]);
        let date = NaiveDate::parse_from_str(&date_text, "%Y-%m-%d")
            .map_err(|_| Error::Parse(format!("日志文件名日期无效: {file_name}")))?;
        out.push(JournalEntry {
            date,
            path: entry.to_string_lossy().into_owned(),
            source,
            content: fs::read_to_string(&entry)?,
        });
    }
    Ok(())
}

/// 层目录下的 `.md` 文件（跳过 README），按路径排序。
fn layer_files(set_root: &Path, layer: &str) -> Result<Vec<PathBuf>, Error> {
    let dir = set_root.join(layer);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files: Vec<PathBuf> = sorted_entries(&dir)?
        .into_iter()
        .filter(|p| {
            p.is_file()
                && p.extension().is_some_and(|ext| ext == "md")
                && file_name(p).as_deref() != Some("README.md")
        })
        .collect();
    files.sort();
    Ok(files)
}

/// 目录条目按路径排序（Dart 的 `listSync` 结果排序后逐个处理）。
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
