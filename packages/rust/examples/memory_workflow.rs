//! memory 工作流演示：状态机四步流程——扫描、过滤、分级、合并。
//!
//! ```sh
//! cargo run --example memory_workflow                # 默认定位主仓库 assets/memory
//! cargo run --example memory_workflow -- <仓库根路径>
//! ```

use std::path::PathBuf;

use quanttide_founder::memory::states::{Destination, Event, MemoryState, decide_merge, grade};
use quanttide_founder::{ClassifiedEntry, InsightGrade, InsightItem, MemoryFlow, MemoryRepository};
// 状态机库经由工具箱再导出，不必自己加 statig 依赖
use quanttide_founder::statig::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = match std::env::args().nth(1) {
        Some(path) => PathBuf::from(path),
        None => default_root(),
    };
    let repo = MemoryRepository::load(&root)?;

    println!("memory 工作流演示");
    println!("根目录：{}", root.display());

    for set in &repo.sets {
        if set.journals.is_empty() {
            continue;
        }
        println!();
        println!("【{}】", set.name);

        // 第一步：扫描——收集日志里的行文本
        let mut texts: Vec<String> = Vec::new();
        for journal in &set.journals {
            for segment in journal.segments() {
                for line in segment.split('\n') {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        texts.push(trimmed.to_string());
                    }
                }
            }
        }

        // 第二步：过滤——文本作为事件进场，状态机做三问路由
        let mut machine = MemoryFlow.state_machine();
        machine.handle(&Event::Classify(texts.clone()));
        let classified = match machine.state() {
            MemoryState::Routed { classified } | MemoryState::Clustered { classified } => {
                classified.clone()
            }
            other => panic!("分类事件应进入 Routed，实际 {other:?}"),
        };

        // 按去向计数（顺序 = 去向首次出现的顺序）
        let mut counts: Vec<(Destination, usize)> = Vec::new();
        for entry in &classified {
            match counts
                .iter_mut()
                .find(|(dest, _)| *dest == entry.destination)
            {
                Some((_, n)) => *n += 1,
                None => counts.push((entry.destination, 1)),
            }
        }

        println!("  扫描：{} 行文本", texts.len());
        println!("  分类：");
        for (destination, n) in &counts {
            println!("    {} → {} 条", destination.as_str(), n);
        }

        // 第三步：证据分级（示例：对前 3 条认知类条目分级）
        let insights: Vec<&ClassifiedEntry> = classified
            .iter()
            .filter(|c| c.destination == Destination::Insight)
            .take(3)
            .collect();
        if !insights.is_empty() {
            println!("  分级（前 {} 条认知类）：", insights.len());
            for item in &insights {
                let graded = grade(&item.text, 1, false, false);
                let label = match graded {
                    InsightGrade::Confirmed => "confirmed",
                    InsightGrade::Hypothesis => "hypothesis",
                };
                println!("    [{label}] {}…", clip(&item.text, 30));
            }
        }

        // 第四步：合并策略（示例：两条相似文本）
        if insights.len() >= 2 {
            let a = InsightItem::new(&insights[0].text, "");
            let b = InsightItem::new(&insights[1].text, "");
            let decision = decide_merge(Some(&a), b);
            println!("  合并：{}（第一条 × 第二条）", decision.action.as_str());
        }
    }
    Ok(())
}

/// 按字符数截断（恒带省略号，同 Dart 示例）。
fn clip(text: &str, width: usize) -> String {
    let flat = text.replace('\n', " ");
    let taken: String = flat.chars().take(width).collect();
    format!("{taken}…")
}

/// 依次尝试：主仓库 assets/memory、当前目录，返回首个存在的候选。
fn default_root() -> PathBuf {
    let candidates = [
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../assets/memory"),
        PathBuf::from("assets/memory"),
    ];
    for candidate in candidates {
        if candidate.is_dir() {
            return candidate.canonicalize().unwrap_or(candidate);
        }
    }
    PathBuf::from("assets/memory")
}
