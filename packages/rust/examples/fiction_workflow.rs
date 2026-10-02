//! fiction 工作流演示：状态机三步提炼、章节编号轴、阶段流转、包装文案。
//!
//! ```sh
//! cargo run --example fiction_workflow                # 默认定位主仓库 assets/fiction
//! cargo run --example fiction_workflow -- <路径>
//! ```
//!
//! 三步提炼与包装文案走 LLM（配置读 quanttide-agent 的环境变量）；
//! 未配置 key 时用演示客户端，判断走显式降级——取样取首句、片段与包装按规则补。

use std::path::PathBuf;

use quanttide_agent::LLMError;
use quanttide_agent::llm::{HttpClient, LLM};
use quanttide_founder::core::engine::Engine;
use quanttide_founder::fiction::states::{
    Event, FictionState, assign_number, check_stage, extract_packaging, find_gaps, stage_flow,
};
use quanttide_founder::{FictionFlow, FictionRepository};
use statig::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = match std::env::args().nth(1) {
        Some(path) => PathBuf::from(path),
        None => default_root(),
    };
    let repo = FictionRepository::load(&root)?;
    let mut engine = demo_engine();

    println!("fiction 工作流演示");
    println!("根目录：{}", root.display());

    // 三步提炼：观察站情绪日记 → 场景素材 + 母题卡片
    println!();
    println!("═══ 三步提炼（观察站 → 创作片段）═══");
    for diary in &repo.observation.emotional_diaries {
        let mut machine = FictionFlow.state_machine();
        machine.handle_with_context(&Event::Sample(diary.clone()), &mut engine);
        machine.handle_with_context(&Event::Expand, &mut engine);
        machine.handle_with_context(&Event::Settle, &mut engine);
        let fragment = match machine.state() {
            FictionState::Fragmented { fragment } => fragment,
            other => panic!("三步走完应有片段，实际 {other:?}"),
        };
        println!("  来源：{}", fragment.source);
        println!("    母题：{}", fragment.motif);
        println!("    场景：{}", fragment.scene);
        println!();
    }

    for novel in &repo.novels {
        println!("═══ {} ═══", novel.name);

        // 编号轴：预留空号
        let gaps = find_gaps(novel);
        if !gaps.is_empty() {
            let gaps: Vec<String> = gaps.iter().map(u32::to_string).collect();
            println!("  预留空号：{}", gaps.join("、"));
        }

        // 分配下一个编号
        let next = assign_number(novel);
        let how = match next.gap {
            Some(gap) => format!("（填空号 {gap}）"),
            None => "（追加）".to_string(),
        };
        println!("  下一个可用编号：{}{}", next.assigned, how);

        // 阶段流转
        println!("  阶段流转：{}", stage_flow(novel).join(" → "));

        // 各阶段完成度
        for stage in &novel.stages {
            let status = check_stage(stage);
            let label = if status.is_complete { "✓" } else { "…" };
            let mut line = format!(
                "    {label} {}_{}：{} 章",
                stage.number, stage.name, status.total_chapters
            );
            if !status.missing_numbers.is_empty() {
                let missing: Vec<String> =
                    status.missing_numbers.iter().map(u32::to_string).collect();
                line.push_str(&format!("，缺 {}", missing.join("、")));
            }
            println!("{line}");
        }

        // 包装文案（取第一个定稿/成稿章节）
        for stage in &novel.stages {
            if (stage.name.contains("定稿") || stage.name.contains("成稿"))
                && let Some(chapter) = stage.chapters.first()
            {
                let packaging = extract_packaging(&engine, &chapter.content);
                println!("  包装文案（{}）：", chapter.title);
                println!("    标题：{}", packaging.title);
                println!("    简介：{}", packaging.tagline);
                println!("    立意：{}", packaging.theme);
                break;
            }
        }
        println!();
    }
    Ok(())
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

/// 演示客户端：不发请求，回的文本解析不出填表结果，降级路径因此触发。
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

/// 依次尝试：主仓库 assets/fiction、当前目录，返回首个存在的候选。
fn default_root() -> PathBuf {
    let candidates = [
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../assets/fiction"),
        PathBuf::from("assets/fiction"),
    ];
    for candidate in candidates {
        if candidate.is_dir() {
            return candidate.canonicalize().unwrap_or(candidate);
        }
    }
    PathBuf::from("assets/fiction")
}
