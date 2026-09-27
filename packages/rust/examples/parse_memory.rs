//! memory 解析引擎演示：装载 `assets/memory` 并输出解析报告。
//!
//! ```sh
//! cargo run --example parse_memory                # 默认定位主仓库 assets/memory
//! cargo run --example parse_memory -- <仓库根路径>
//! ```

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use quanttide_founder::memory::models::JournalSource;
use quanttide_founder::{InsightGrade, MemoryRepository, MemorySet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = match std::env::args().nth(1) {
        Some(path) => PathBuf::from(path),
        None => default_root(),
    };
    let repo = MemoryRepository::load(&root)?;

    println!("memory 解析报告");
    println!("根目录：{}", root.display());
    for set in &repo.sets {
        print_set(set);
    }

    let journals: usize = repo.sets.iter().map(|s| s.journals.len()).sum();
    let profiles: usize = repo.sets.iter().map(|s| s.profiles.len()).sum();
    let insights: usize = repo.sets.iter().map(|s| s.insights.len()).sum();
    let roadmaps: usize = repo.sets.iter().map(|s| s.roadmaps.len()).sum();
    println!();
    println!(
        "合计：{} 个记忆集 / {} 篇日志 / {} 份档案 / {} 份洞察 / {} 份路线图",
        repo.sets.len(),
        journals,
        profiles,
        insights,
        roadmaps
    );
    Ok(())
}

fn print_set(set: &MemorySet) {
    println!();
    println!("【{}】", set.name);

    if !set.journals.is_empty() {
        let characters: usize = set.journals.iter().map(|j| j.character_count()).sum();
        let newest = set.journals.first().expect("非空").date;
        let oldest = set.journals.last().expect("非空").date;
        println!(
            "  时间线：{} 篇（{} ~ {}），{} 字",
            set.journals.len(),
            format_date(oldest),
            format_date(newest),
            characters
        );
        for journal in &set.journals {
            let location = match journal.source {
                JournalSource::Root => "集根",
                JournalSource::Archive => "journal",
            };
            let segments = journal.segments();
            let first_line = segments
                .first()
                .and_then(|segment| segment.split('\n').next())
                .unwrap_or("");
            println!(
                "    {}  {}  {} 段  {}",
                format_date(journal.date),
                location,
                segments.len(),
                clip(first_line, 28)
            );
        }
    }

    if !set.profiles.is_empty() {
        println!("  档案：{} 份", set.profiles.len());
        for profile in &set.profiles {
            let titles: Vec<String> = profile
                .sections
                .iter()
                .map(|s| clip(&s.title, 10))
                .collect();
            println!(
                "    {}（{} 节）：{}",
                profile.title.as_deref().unwrap_or(&profile.name),
                profile.sections.len(),
                titles.join("、")
            );
        }
    }

    if !set.insights.is_empty() {
        println!("  洞察：{} 份", set.insights.len());
        for insight in &set.insights {
            let confirmed = insight.items_of(InsightGrade::Confirmed).count();
            let hypothesis = insight.items_of(InsightGrade::Hypothesis).count();
            let mut parts: Vec<String> = Vec::new();
            if confirmed > 0 {
                parts.push(format!("已确认 {confirmed} 条"));
            }
            if hypothesis > 0 {
                parts.push(format!("假说 {hypothesis} 条"));
            }
            let label = insight.title.as_deref().unwrap_or(&insight.name);
            if parts.is_empty() {
                let prose: usize = insight.sections.iter().map(|s| s.paragraphs.len()).sum();
                println!("    {label}：主题式 {prose} 段");
            } else {
                println!("    {label}：{}", parts.join("，"));
            }
        }
    }

    if !set.roadmaps.is_empty() {
        println!("  路线图：{} 份", set.roadmaps.len());
        for roadmap in &set.roadmaps {
            let mut parts: Vec<String> = Vec::new();
            if let Some(goal) = &roadmap.goal {
                parts.push(format!("目标「{}」", clip(goal, 20)));
            }
            count(&mut parts, "元目标", roadmap.meta_goals.len());
            count(&mut parts, "核心问题", roadmap.core_problems.len());
            count(&mut parts, "已决策", roadmap.decided.len());
            count(&mut parts, "待决策", roadmap.pending.len());
            count(&mut parts, "主题节", roadmap.theme_sections.len());
            let label = roadmap.title.as_deref().unwrap_or(&roadmap.name);
            println!("    {label}：{}", parts.join("，"));
        }
    }
}

fn count(parts: &mut Vec<String>, label: &str, n: usize) {
    if n > 0 {
        parts.push(format!("{label} {n}"));
    }
}

fn format_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// 压成一行并按字符数截断，超长加省略号。
fn clip(text: &str, width: usize) -> String {
    let flat = text.replace('\n', " ");
    if flat.chars().count() <= width {
        flat
    } else {
        let mut clipped: String = flat.chars().take(width).collect();
        clipped.push('…');
        clipped
    }
}

/// 依次尝试：主仓库 assets/memory、当前目录，返回首个存在的候选。
fn default_root() -> PathBuf {
    let candidates = [
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../assets/memory"),
        PathBuf::from("assets/memory"),
    ];
    for candidate in candidates {
        if candidate.is_dir() {
            return candidate.canonicalize().unwrap_or(candidate);
        }
    }
    PathBuf::from("assets/memory")
}
