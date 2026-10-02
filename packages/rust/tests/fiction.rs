//! fiction 域：仓库装载 + 三步提炼状态机 + 编号轴/包装文案。
//!
//! 语义步骤走 LLM 首选（`semantic_llm`）；规则降级用 `demo_llm`（判断必然失败）。

mod common;

use common::{demo_llm, fiction_fixture, semantic_llm};
use quanttide_founder::Error;
use quanttide_founder::core::engine::Engine;
use quanttide_founder::fiction::states::{
    Event, FictionFlow, FictionState, assign_number, check_stage, extract_packaging, find_gaps,
    packaging_by_rules, stage_flow,
};
use quanttide_founder::{EmotionalDiary, FictionRepository};
use statig::prelude::*;

fn load_fixture() -> (tempfile::TempDir, FictionRepository) {
    let dir = tempfile::tempdir().expect("建临时目录");
    fiction_fixture(dir.path());
    let repo = FictionRepository::load(dir.path()).expect("装载仓库");
    (dir, repo)
}

fn semantic_engine() -> Engine {
    Engine::new(semantic_llm())
}

// ---------------------------------------------------------------------------
// 仓库装载
// ---------------------------------------------------------------------------

#[test]
fn discovers_novels_stages_and_chapters() {
    let (_dir, repo) = load_fixture();
    assert_eq!(repo.novels.len(), 1); // 实验室/ 与 杂物/ 不算小说

    let novel = &repo.novels[0];
    assert_eq!(novel.name, "测试小说");
    assert!(
        novel
            .index_content
            .as_deref()
            .unwrap_or("")
            .contains("晋江发文资料")
    );

    let stage_names: Vec<&str> = novel.stages.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(stage_names, vec!["灵感", "初稿"]);

    let first = &novel.stages[0];
    assert_eq!(first.number, 1);
    assert_eq!(first.chapters.len(), 3);
    // 章节按编号排序：0_前言 → 1_开头 → 3_转折
    assert_eq!(first.chapters[0].number, Some(0));
    assert!(first.chapters[0].is_preface());
    assert_eq!(first.chapters[1].number, Some(1));
    assert_eq!(first.chapters[1].title, "开头");
    assert_eq!(first.chapters[2].number, Some(3));

    let second = &novel.stages[1];
    assert_eq!(second.chapters.len(), 2); // 1_开头 + 地摊火锅（未编号），README 不收
    assert!(second.chapters[1].is_unnumbered());
    assert_eq!(second.chapters[1].title, "地摊火锅");

    // 全部章节跨阶段共用编号轴：0 / 1 / 1 / 3 / 未编号
    let numbers: Vec<Option<u32>> = novel.all_chapters().iter().map(|c| c.number).collect();
    assert_eq!(numbers, vec![Some(0), Some(1), Some(1), Some(3), None]);
}

#[test]
fn discovers_observation_station() {
    let (_dir, repo) = load_fixture();
    assert_eq!(repo.observation.emotional_diaries.len(), 1); // README 不收
    assert_eq!(repo.observation.emotional_diaries[0].title, "深夜地铁");
    assert_eq!(repo.observation.social_observations.len(), 1);
    assert_eq!(repo.observation.social_observations[0].title, "排队");
}

#[test]
fn missing_root_reports_domain() {
    let err = FictionRepository::load("/definitely/not/here").expect_err("根目录不存在");
    assert!(matches!(
        err,
        Error::RootNotFound {
            domain: "fiction",
            ..
        }
    ));
    assert!(err.to_string().contains("fiction 仓库根目录不存在"));
}

// ---------------------------------------------------------------------------
// 三步提炼状态机
// ---------------------------------------------------------------------------

fn sample_diary() -> EmotionalDiary {
    EmotionalDiary {
        title: "深夜地铁".to_string(),
        file: "观察站/1_情绪日记/深夜地铁.md".into(),
        content: "心里发闷，堵得慌。\n在地铁上看着拿手机的人，忽然想写一个赶末班车的角色。\n"
            .to_string(),
    }
}

#[test]
fn events_walk_idle_sampled_observed_fragmented() {
    let diary = sample_diary();
    let mut engine = semantic_engine();
    let mut machine = FictionFlow.state_machine();

    // 来早了：没取样就落实，不理会
    machine.handle_with_context(&Event::Settle, &mut engine);
    assert!(matches!(machine.state(), FictionState::Idle {}));
    assert!(machine.state().fragment().is_none());

    machine.handle_with_context(&Event::Sample(diary.clone()), &mut engine);
    let (sample, source) = match machine.state() {
        FictionState::Sampled { sample, source } => (sample.clone(), source.clone()),
        other => panic!("应处于 Sampled，实际 {other:?}"),
    };
    // 取样按判据挑出含具体细节的句子（第一句只有情绪，没有动作）
    assert!(sample.contains("看着拿手机的人"));
    assert_eq!(source, "深夜地铁");

    // 没观察展开就不给片段
    machine.handle_with_context(&Event::Settle, &mut engine);
    assert!(matches!(machine.state(), FictionState::Sampled { .. }));
    assert!(machine.state().fragment().is_none());

    machine.handle_with_context(&Event::Expand, &mut engine);
    let observation = {
        let (observation, _) = machine.state().observation().expect("已观察");
        assert_eq!(observation, sample);
        observation.to_string()
    };
    machine.handle_with_context(&Event::Settle, &mut engine);

    let fragment = machine.state().fragment().expect("已提炼");
    // 母题与场景由 LLM 按 fragment.yaml 填表；source 是素材标题
    assert_eq!(fragment.motif, "赶末班车的角色");
    assert_eq!(fragment.scene, "她把票塞进他手里");
    assert_eq!(fragment.source, "深夜地铁");
    assert_eq!(observation, sample);
}

#[test]
fn extract_runs_all_three_steps() {
    let diary = sample_diary();
    let mut engine = semantic_engine();
    let fragment = FictionFlow::extract(&mut engine, &diary);
    assert_eq!(fragment.motif, "赶末班车的角色");
    assert_eq!(fragment.scene, "她把票塞进他手里");

    let fragments = FictionFlow::extract_all(&mut engine, &[diary]);
    assert_eq!(fragments.len(), 1);
}

#[test]
fn extract_falls_back_to_rules_without_llm() {
    // 判断失败（LLM 不可用）→ 规则降级：取样取首句、片段母题 = 素材标题、场景截 50 字
    let diary = EmotionalDiary {
        title: "长文".to_string(),
        file: "a.md".into(),
        content: format!("走看{}", "很多".repeat(30)),
    };
    let mut engine = Engine::new(demo_llm());
    let fragment = FictionFlow::extract(&mut engine, &diary);
    assert_eq!(fragment.motif, "长文");
    assert_eq!(fragment.scene.chars().count(), 51); // 50 字 + 省略号
    assert!(fragment.scene.ends_with('…'));
}

// ---------------------------------------------------------------------------
// 编号轴 / 阶段流转 / 包装文案
// ---------------------------------------------------------------------------

#[test]
fn numbering_axis_finds_gaps_and_assigns() {
    let (_dir, repo) = load_fixture();
    let novel = &repo.novels[0];
    // 正文用号 1、3（前言 0_ 不占号）→ 空号 2
    assert_eq!(find_gaps(novel), vec![2]);

    let assignment = assign_number(novel);
    assert_eq!(assignment.assigned, 2);
    assert_eq!(assignment.gap, Some(2)); // 宁空勿移，优先填空号

    let stage_flow = stage_flow(novel);
    assert_eq!(stage_flow, vec!["1_灵感", "2_初稿"]);
}

#[test]
fn stage_completion_tracks_missing_numbers() {
    let (_dir, repo) = load_fixture();
    let novel = &repo.novels[0];
    let status = check_stage(&novel.stages[0]); // 1_灵感：1、3 编号，缺 2
    assert!(!status.is_complete);
    assert_eq!(status.missing_numbers, vec![2]);
    assert_eq!(status.total_chapters, 3);

    let draft = check_stage(&novel.stages[1]); // 2_初稿：1 已编号，无空位
    assert!(draft.is_complete);
    assert!(draft.missing_numbers.is_empty());
    assert_eq!(draft.total_chapters, 2);
}

#[test]
fn packaging_prefers_llm_then_falls_back_to_rules() {
    // LLM 首选：按 packaging.yaml 填表
    let engine = semantic_engine();
    let packaging = extract_packaging(&engine, "她把票塞进他手里。夜班地铁的灯忽明忽暗。");
    assert_eq!(packaging.title, "她把票塞进他手里");
    assert_eq!(packaging.tagline, "夜班地铁的灯忽明忽暗");
    assert_eq!(packaging.theme, "错过与递出");

    // 规则降级：首句作标题、5-15 字句作简介、末句作立意
    let fallback = Engine::new(demo_llm());
    let packaging = extract_packaging(&fallback, "她把票塞进他手里。夜班地铁的灯忽明忽暗。");
    assert_eq!(packaging.title, "她把票塞进他手里");
    assert_eq!(packaging.tagline, "她把票塞进他手里"); // 8 字，落在 5..=15
    assert_eq!(packaging.theme, "夜班地铁的灯忽明忽暗");

    // 没有 5..=15 字的句子：简介留空
    let packaging = packaging_by_rules("短。这一句特别特别特别特别特别特别长。");
    assert_eq!(packaging.tagline, "");

    // 空正文：三样都空
    assert_eq!(packaging_by_rules(""), Default::default());
}
