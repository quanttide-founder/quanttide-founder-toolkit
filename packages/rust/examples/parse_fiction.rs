//! fiction 解析引擎演示：装载 fiction 仓库并输出解析报告。
//!
//! ```sh
//! cargo run --example parse_fiction                # 默认定位主仓库 assets/fiction
//! cargo run --example parse_fiction -- <仓库根路径>
//! ```

use std::path::{Path, PathBuf};

use quanttide_founder::{FictionRepository, Novel, Observation};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = match std::env::args().nth(1) {
        Some(path) => PathBuf::from(path),
        None => default_root(),
    };
    let repo = FictionRepository::load(&root)?;

    println!("fiction 解析报告");
    println!("根目录：{}", root.display());
    for novel in &repo.novels {
        print_novel(novel);
    }
    print_observation(&repo.observation);

    let total_chapters: usize = repo.novels.iter().map(|n| n.all_chapters().len()).sum();
    println!();
    println!(
        "合计：{} 部小说 / {} 个章节 / {} 篇情绪日记 / {} 条社会观察",
        repo.novels.len(),
        total_chapters,
        repo.observation.emotional_diaries.len(),
        repo.observation.social_observations.len()
    );
    Ok(())
}

fn print_novel(novel: &Novel) {
    println!();
    println!("【{}】", novel.name);

    // 晋江资料（index.md）
    if let Some(index) = &novel.index_content {
        println!(
            "  晋江资料：{}",
            extract_title(index).unwrap_or("（未提取标题）")
        );
    }

    // 创作阶段
    for stage in &novel.stages {
        let numbered: Vec<&quanttide_founder::Chapter> = stage
            .chapters
            .iter()
            .filter(|c| c.number.is_some())
            .collect();
        let unnumbered: Vec<&quanttide_founder::Chapter> = stage
            .chapters
            .iter()
            .filter(|c| c.number.is_none())
            .collect();
        let prefaces: Vec<&quanttide_founder::Chapter> = numbered
            .iter()
            .copied()
            .filter(|c| c.is_preface())
            .collect();

        let mut parts: Vec<String> = Vec::new();
        if !numbered.is_empty() {
            let mut numbers: Vec<u32> =
                numbered.iter().map(|c| c.number.expect("已编号")).collect();
            numbers.sort_unstable();
            parts.push(format!(
                "{} 章（编号 {}–{}）",
                numbered.len(),
                numbers.first().expect("非空"),
                numbers.last().expect("非空")
            ));
        }
        if !prefaces.is_empty() {
            parts.push(format!("{} 篇前言", prefaces.len()));
        }
        if !unnumbered.is_empty() {
            parts.push(format!("{} 篇未编号", unnumbered.len()));
        }
        println!("  {}_{}：{}", stage.number, stage.name, parts.join("，"));

        for chapter in &stage.chapters {
            let label = match chapter.number {
                Some(number) => format!("{number}_"),
                None => "未编号 ".to_string(),
            };
            let chars = chapter
                .content
                .chars()
                .filter(|c| !c.is_whitespace())
                .count();
            println!("    {}{}（{} 字）", label, chapter.title, chars);
        }
    }

    // 编号覆盖情况
    let numbers: Vec<u32> = novel
        .all_chapters()
        .iter()
        .filter(|c| c.number.is_some() && !c.is_preface())
        .filter_map(|c| c.number)
        .collect();
    if !numbers.is_empty() {
        let max = numbers.iter().copied().max().expect("非空");
        let gaps: Vec<String> = (1..=max)
            .filter(|n| !numbers.contains(n))
            .map(|n| n.to_string())
            .collect();
        if !gaps.is_empty() {
            println!("  预留空号：{}", gaps.join("、"));
        }
    }
}

fn print_observation(obs: &Observation) {
    if obs.emotional_diaries.is_empty() && obs.social_observations.is_empty() {
        return;
    }

    println!();
    println!("【观察站】");

    if !obs.emotional_diaries.is_empty() {
        println!("  情绪日记：{} 篇", obs.emotional_diaries.len());
        for diary in &obs.emotional_diaries {
            let chars = diary.content.chars().filter(|c| !c.is_whitespace()).count();
            println!("    {}（{} 字）", diary.title, chars);
        }
    }

    if !obs.social_observations.is_empty() {
        println!("  社会观察：{} 条", obs.social_observations.len());
        for observation in &obs.social_observations {
            let chars = observation
                .content
                .chars()
                .filter(|c| !c.is_whitespace())
                .count();
            println!("    {}（{} 字）", observation.title, chars);
        }
    }
}

fn extract_title(index_content: &str) -> Option<&str> {
    index_content
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("# "))
        .map(|line| &line[2..])
}

/// 依次尝试：主仓库 assets/fiction、当前目录，返回首个存在的候选。
fn default_root() -> PathBuf {
    let candidates = [
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../assets/fiction"),
        PathBuf::from("assets/fiction"),
    ];
    for candidate in candidates {
        if candidate.is_dir() {
            return candidate.canonicalize().unwrap_or(candidate);
        }
    }
    PathBuf::from("assets/fiction")
}
