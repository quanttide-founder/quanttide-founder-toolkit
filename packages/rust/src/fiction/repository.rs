//! fiction/repository：仓库装载——发现小说与观察站、逐层读文件。
//!
//! 仓库结构：
//! ```text
//! fiction/
//! ├── {小说名}/           每部小说一个目录
//! │   ├── index.md        晋江发文资料
//! │   └── {N}_{阶段}/     创作阶段（1_灵感/2_场景/…）
//! ├── 观察站/
//! │   ├── 1_情绪日记/
//! │   └── 2_社会观察/
//! └── 实验室/             AI 产物，由使用方单独处理
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::fiction::models::{EmotionalDiary, Novel, Observation, SocialObservation, Stage};

/// fiction 仓库的装载结果。
#[derive(Debug, Clone)]
pub struct FictionRepository {
    pub root: PathBuf,
    pub novels: Vec<Novel>,
    pub observation: Observation,
}

impl FictionRepository {
    /// 装载仓库。小说目录 = 含 `index.md` 或 `N_阶段` 子目录的目录。
    pub fn load(root: impl AsRef<Path>) -> Result<Self, Error> {
        let root = root.as_ref();
        if !root.is_dir() {
            return Err(Error::RootNotFound {
                domain: "fiction",
                path: root.display().to_string(),
            });
        }

        let mut novels: Vec<Novel> = Vec::new();
        let mut observation: Option<Observation> = None;

        let mut entries: Vec<PathBuf> = fs::read_dir(root)?
            .map(|entry| entry.map(|e| e.path()))
            .collect::<Result<_, _>>()?;
        entries.sort();

        for entry in entries {
            if !entry.is_dir() {
                continue;
            }
            let Some(name) = file_name(&entry) else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }

            if name == "观察站" {
                observation = Some(parse_observation(&entry)?);
                continue;
            }
            if name == "实验室" {
                continue; // 实验室由使用方单独处理
            }

            // 判断是否为小说目录：含 index.md 或 N_ 阶段子目录
            let has_index = entry.join("index.md").is_file();
            let has_stage = fs::read_dir(&entry)?
                .map(|e| e.map(|e| e.path()))
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .filter(|p| p.is_dir())
                .any(|p| file_name(p).is_some_and(|n| Stage::is_stage_dir_name(&n)));
            if !has_index && !has_stage {
                continue;
            }

            let index_file = entry.join("index.md");
            novels.push(Novel {
                name,
                stages: Stage::discover(&entry)?,
                directory: entry,
                index_content: if index_file.is_file() {
                    Some(fs::read_to_string(&index_file)?)
                } else {
                    None
                },
            });
        }

        Ok(FictionRepository {
            root: root.to_path_buf(),
            novels,
            observation: observation.unwrap_or_default(),
        })
    }
}

/// 观察站：情绪日记（`1_情绪日记/`）+ 社会观察（`2_社会观察/`）。
fn parse_observation(dir: &Path) -> Result<Observation, Error> {
    let emotional_diaries = read_md(&dir.join("1_情绪日记"))?
        .into_iter()
        .map(|(title, file, content)| EmotionalDiary {
            title,
            file,
            content,
        })
        .collect();
    let social_observations = read_md(&dir.join("2_社会观察"))?
        .into_iter()
        .map(|(title, file, content)| SocialObservation {
            title,
            file,
            content,
        })
        .collect();
    Ok(Observation {
        emotional_diaries,
        social_observations,
    })
}

/// 读一个目录下的 `.md` 文件（跳过 README），返回 (标题, 路径, 原文)。
fn read_md(dir: &Path) -> Result<Vec<(String, PathBuf, String)>, Error> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<_, _>>()?;
    entries.sort();

    let mut items = Vec::new();
    for entry in entries {
        if !entry.is_file() {
            continue;
        }
        let Some(name) = file_name(&entry) else {
            continue;
        };
        if !name.ends_with(".md") || name == "README.md" {
            continue;
        }
        items.push((
            name[..name.len() - 3].to_string(),
            entry.clone(),
            fs::read_to_string(&entry)?,
        ));
    }
    Ok(items)
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}
