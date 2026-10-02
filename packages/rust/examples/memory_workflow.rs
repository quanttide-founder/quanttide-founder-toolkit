//! memory 工作流演示：状态机四步流程——扫描、分类、分级、合并。
//!
//! ```sh
//! cargo run --example memory_workflow                # 默认定位主仓库 assets/memory
//! cargo run --example memory_workflow -- <仓库根路径>
//! ```
//!
//! 分类与分级走 LLM（配置读 quanttide-agent 的环境变量 LLM_MODEL / LLM_BASE_URL /
//! LLM_API_KEY）；未配置 key 时用演示客户端，各步走显式降级——分类留在 journal 并写明失败原因。

use std::path::PathBuf;

use quanttide_agent::LLMError;
use quanttide_agent::llm::{HttpClient, LLM};
use quanttide_founder::core::engine::Engine;
use quanttide_founder::memory::states::{Event, MemoryState, decide_merge, grade};
use quanttide_founder::{ClassifiedEntry, InsightItem, MemoryFlow, MemoryRepository};
// 状态机库经由工具箱再导出，不必自己加 statig 依赖
use quanttide_founder::statig::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = match std::env::args().nth(1) {
        Some(path) => PathBuf::from(path),
        None => default_root(),
    };
    let repo = MemoryRepository::load(&root)?;
    let mut engine = demo_engine();

    println!("memory 工作流演示");
    println!("根目录：{}", root.display());

    for set in &repo.sets {
        if set.journals.is_empty() {
            continue;
        }
        println!();
        println!("【{}】", set.name);

        // 第一步：扫描——收集日志里的内容行（结构行不收）
        let mut texts: Vec<String> = Vec::new();
        for journal in &set.journals {
            for segment in journal.segments() {
                for line in segment.split('\n') {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        texts.push(trimmed.to_string());
                    }
                }
            }
        }

        // 第二步：分类——文本作为事件进场，状态机按类别判据路由。
        // 演示只判前 10 行：全量逐行判断是真实 LLM 调用，跑起来以分钟计
        let total = texts.len();
        texts.truncate(10);
        let mut machine = MemoryFlow.state_machine();
        machine.handle_with_context(&Event::Classify(texts.clone()), &mut engine);
        let classified = match machine.state() {
            MemoryState::Routed { classified } | MemoryState::Clustered { classified } => {
                classified.clone()
            }
            other => panic!("分类事件应进入 Routed，实际 {other:?}"),
        };

        // 按去向计数（顺序 = 去向首次出现的顺序）
        let mut counts: Vec<(&str, usize)> = Vec::new();
        for entry in &classified {
            match counts
                .iter_mut()
                .find(|(name, _)| *name == entry.destination_name())
            {
                Some((_, n)) => *n += 1,
                None => counts.push((entry.destination_name(), 1)),
            }
        }

        println!("  扫描：{total} 行文本（演示只判前 {} 行）", texts.len());
        println!("  分类：");
        for (destination, n) in &counts {
            println!("    {} → {} 条", destination, n);
        }
        // 判断理由必填：LLM 不可用时这里能看到失败原因
        for entry in classified
            .iter()
            .filter(|e| e.destination.is_none())
            .take(1)
        {
            println!("  示例理由：{}", entry.reason);
        }

        // 第三步：证据分级（示例：对前 3 条认知类条目分级）
        let insights: Vec<&ClassifiedEntry> = classified
            .iter()
            .filter(|c| c.destination_name() == "insight")
            .take(3)
            .collect();
        if !insights.is_empty() {
            println!("  分级（前 {} 条认知类）：", insights.len());
            for item in &insights {
                let graded = grade(&engine, &item.text, 1, false, false);
                println!("    [{}] {}…", graded, clip(&item.text, 30));
            }
        }

        // 第四步：合并策略（示例：两条相似文本）
        if insights.len() >= 2 {
            let a = InsightItem::new(&insights[0].text, "");
            let b = InsightItem::new(&insights[1].text, "");
            let decision = decide_merge(&engine, Some(&a), b);
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

/// 有 API key 才真调 LLM；未配置就用演示客户端，判断走显式降级（示例离线可跑）。
fn demo_engine() -> Engine {
    let configured = std::env::var("LLM_API_KEY")
        .or_else(|_| std::env::var("DEEPSEEK_API_KEY"))
        .is_ok();
    if configured {
        return Engine::new(LLM::default());
    }
    println!("（未配置 LLM_API_KEY——判断走规则降级）");
    Engine::new(LLM::with_client(
        "demo",
        "http://localhost",
        "",
        Box::new(OfflineLlm),
    ))
}

/// 演示客户端：不发请求，回的文本解析不出判断，降级路径因此触发。
struct OfflineLlm;

impl HttpClient for OfflineLlm {
    fn post_json(
        &self,
        _url: &str,
        _auth: &str,
        _body: &serde_json::Value,
    ) -> Result<serde_json::Value, LLMError> {
        Ok(serde_json::json!({
            "model": "demo",
            "choices": [{
                "message": { "role": "assistant", "content": "（演示模式，未调用 LLM）" },
                "finish_reason": "stop"
            }]
        }))
    }
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
