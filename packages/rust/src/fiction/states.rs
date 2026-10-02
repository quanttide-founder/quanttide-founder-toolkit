//! fiction/states：三步提炼的状态机（statig）+ 结构判断。
//!
//! 事件进 → 状态出：
//!
//! ```text
//! Idle ──Sample──▶ Sampled ──Expand──▶ Observed ──Settle──▶ Fragmented
//! ```
//!
//! 取样、观察展开、落实片段每步只做一件事，中间产物放在当前状态里；
//! 直接从情绪跳到场景会产出「金句」而不是「画面」——不到 Observed 就不给 Settle。
//!
//! 语义步骤（取样、观察展开、落实片段、包装文案）交给 `Engine`：LLM 按
//! [`crate::fiction::rules`] 的判据首选，规则降级显式跟在后面；判断随上下文进场。
//! 章节编号轴、阶段流转是纯结构判断，不随状态走，收成本模块的自由函数。
//! 对照 Dart 包的 `fiction/bloc.dart`：bloc 换成 statig 状态机，规则逐条对齐。

use std::collections::HashSet;

use serde_json::Value;
use statig::prelude::*;

use crate::core::engine::{Engine, RuleBasedExtractor, SemanticExtractor};
use crate::core::parse::MarkdownDocument;
use crate::fiction::models::{EmotionalDiary, Novel, Stage};
use crate::fiction::rules::{FRAGMENT, OBSERVATION, PACKAGING};

// ---------------------------------------------------------------------------
// 状态机：三步提炼
// ---------------------------------------------------------------------------

/// fiction 三步提炼的状态机（无共享存储，中间产物都在状态里；语义步骤走上下文的 Engine）。
pub struct FictionFlow;

/// 状态机的事件：谁触发了流程、带什么进场。
#[derive(Debug)]
pub enum Event {
    /// 取样：从情绪记录定位可写素材。
    Sample(EmotionalDiary),
    /// 观察展开：从情绪提取外部观察（场景/动作/细节）。
    Expand,
    /// 落实片段：观察 → 场景素材 + 母题卡片。
    Settle,
}

#[state_machine(
    initial = "FictionState::idle()",
    state(name = "FictionState", derive(Debug))
)]
impl FictionFlow {
    #[state]
    fn idle(context: &mut Engine, event: &Event) -> Outcome<FictionState> {
        match event {
            Event::Sample(diary) => {
                let sample = take_sample(context, &diary.content);
                Transition(FictionState::sampled(sample, diary.title.clone()))
            }
            // 还没取样
            Event::Expand | Event::Settle => Handled,
        }
    }

    // 状态局部存储的持有者必须是 &mut Vec/&mut String，不能换成切片
    #[allow(clippy::ptr_arg)]
    #[state]
    fn sampled(
        sample: &mut String,
        source: &mut String,
        context: &mut Engine,
        event: &Event,
    ) -> Outcome<FictionState> {
        match event {
            Event::Expand => {
                let observation = expand_observation(context, sample);
                Transition(FictionState::observed(observation, source.clone()))
            }
            Event::Sample(diary) => {
                let sample = take_sample(context, &diary.content);
                Transition(FictionState::sampled(sample, diary.title.clone()))
            }
            // 没观察展开就不落实
            Event::Settle => Handled,
        }
    }

    // 状态局部存储的持有者必须是 &mut Vec/&mut String，不能换成切片
    #[allow(clippy::ptr_arg)]
    #[state]
    fn observed(
        observation: &mut String,
        source: &mut String,
        context: &mut Engine,
        event: &Event,
    ) -> Outcome<FictionState> {
        match event {
            Event::Settle => {
                let fragment = to_fragment(context, observation, source);
                Transition(FictionState::fragmented(fragment))
            }
            Event::Sample(diary) => {
                let sample = take_sample(context, &diary.content);
                Transition(FictionState::sampled(sample, diary.title.clone()))
            }
            // 观察还没展开
            Event::Expand => Handled,
        }
    }

    #[state]
    fn fragmented(
        fragment: &mut ExtractedFragment,
        context: &mut Engine,
        event: &Event,
    ) -> Outcome<FictionState> {
        match event {
            // 换素材：重新取样，流程从头走
            Event::Sample(diary) => {
                let sample = take_sample(context, &diary.content);
                Transition(FictionState::sampled(sample, diary.title.clone()))
            }
            // 已提炼完，结果带在状态里
            Event::Expand | Event::Settle => Transition(FictionState::fragmented(fragment.clone())),
        }
    }
}

impl FictionFlow {
    /// 三步提炼一步到位：取样 → 观察展开 → 落实片段（对照 Dart 的 `extract`）。
    ///
    /// 语义步骤需要引擎，判断器随上下文进场。
    pub fn extract(engine: &mut Engine, diary: &EmotionalDiary) -> ExtractedFragment {
        let mut machine = FictionFlow.state_machine();
        machine.handle_with_context(&Event::Sample(diary.clone()), engine);
        machine.handle_with_context(&Event::Expand, engine);
        machine.handle_with_context(&Event::Settle, engine);
        machine
            .state()
            .fragment()
            .cloned()
            .unwrap_or_else(|| ExtractedFragment {
                motif: String::new(),
                scene: String::new(),
                source: String::new(),
            })
    }

    /// 批量提炼。
    pub fn extract_all(engine: &mut Engine, diaries: &[EmotionalDiary]) -> Vec<ExtractedFragment> {
        diaries
            .iter()
            .map(|diary| Self::extract(engine, diary))
            .collect()
    }
}

impl FictionState {
    /// 当前状态携带的中间产物（仅 Observed）。
    pub fn observation(&self) -> Option<(&str, &str)> {
        match self {
            FictionState::Observed {
                observation,
                source,
            } => Some((observation.as_str(), source.as_str())),
            _ => None,
        }
    }

    /// 当前状态携带的提炼结果（仅 Fragmented）。
    pub fn fragment(&self) -> Option<&ExtractedFragment> {
        match self {
            FictionState::Fragmented { fragment } => Some(fragment),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// 结果类型
// ---------------------------------------------------------------------------

/// 提炼结果：母题卡片 + 场景素材。
#[derive(Debug, Clone)]
pub struct ExtractedFragment {
    /// 母题卡片：一句话说清写什么。
    pub motif: String,
    /// 场景素材：50 字钩子。
    pub scene: String,
    /// 来源素材标题。
    pub source: String,
}

/// 编号分配结果。
#[derive(Debug, Clone)]
pub struct NumberAssignment {
    /// 分配到的编号。
    pub assigned: u32,
    /// 填补的空号（None = 追加到最大号 +1）。
    pub gap: Option<u32>,
}

/// 阶段完成状态。
#[derive(Debug, Clone)]
pub struct StageStatus {
    pub is_complete: bool,
    pub missing_numbers: Vec<u32>,
    pub total_chapters: usize,
}

/// 包装文案（晋江规范）：三类文案各自承担不同信息，不互相重复。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Packaging {
    /// 标题 = 核心矛盾与钩子。
    pub title: String,
    /// 一句话简介 = 增量信息（≤15 字）。
    pub tagline: String,
    /// 立意 = 价值维度。
    pub theme: String,
}

// ---------------------------------------------------------------------------
// 结构判断：纯函数，不随状态走
// ---------------------------------------------------------------------------

/// 分配新章节编号：优先填补空号（宁空勿移），否则当前最大号 +1。
pub fn assign_number(novel: &Novel) -> NumberAssignment {
    let used = used_numbers(novel);
    let Some(&max) = used.iter().max() else {
        return NumberAssignment {
            assigned: 1,
            gap: None,
        };
    };
    for candidate in 1..=max {
        if !used.contains(&candidate) {
            return NumberAssignment {
                assigned: candidate,
                gap: Some(candidate),
            };
        }
    }
    NumberAssignment {
        assigned: max + 1,
        gap: None,
    }
}

/// 检查编号轴：返回预留空号列表。
pub fn find_gaps(novel: &Novel) -> Vec<u32> {
    gaps_of(&used_numbers(novel))
}

/// 检查阶段完成度：该阶段的章节都已编号且无空位。
pub fn check_stage(stage: &Stage) -> StageStatus {
    let used: HashSet<u32> = stage
        .chapters
        .iter()
        .filter(|c| c.number.is_some() && !c.is_preface())
        .filter_map(|c| c.number)
        .collect();
    let missing = gaps_of(&used);
    StageStatus {
        is_complete: missing.is_empty() && !used.is_empty(),
        missing_numbers: missing,
        total_chapters: stage.chapters.len(),
    }
}

/// 发现小说的阶段流转方向：按编号排序，返回阶段名列表。
pub fn stage_flow(novel: &Novel) -> Vec<String> {
    novel
        .stages
        .iter()
        .map(|stage| format!("{}_{}", stage.number, stage.name))
        .collect()
}

/// 从正文提取包装文案：标题、一句话简介、立意（LLM 按 `packaging.yaml` 填表，首选）。
///
/// 三类文案各自承担不同信息，不互相重复：
/// 标题 = 核心矛盾与钩子；一句话简介 = 增量信息（≤15 字）；立意 = 价值维度。
/// 字段缺失的按规则降级补（[`packaging_by_rules`]）。
pub fn extract_packaging(engine: &Engine, content: &str) -> Packaging {
    let fallback = packaging_by_rules(content);
    let doc = MarkdownDocument::parse("packaging.md", content);
    let sections = PACKAGING.split(&doc.blocks);
    let Ok(result) = engine.extract(&sections, &PACKAGING) else {
        return fallback;
    };
    Packaging {
        title: field_text(&result.fields, "title").unwrap_or(fallback.title),
        tagline: field_text(&result.fields, "tagline").unwrap_or(fallback.tagline),
        theme: field_text(&result.fields, "theme").unwrap_or(fallback.theme),
    }
}

/// 规则降级：按句子位置取文案——首句作标题、5-15 字句作简介、末句作立意。
pub fn packaging_by_rules(content: &str) -> Packaging {
    // 从正文提取具体细节（台词、场景、意象）做钩子
    let sentences: Vec<&str> = content
        .split(['。', '！', '？', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    // 标题：找有画面感或反转的长句——首句优先
    let title = sentences.first().copied().unwrap_or_default().to_string();

    // 一句话简介：回答「凭什么看」，不超过 15 字
    let tagline = sentences
        .iter()
        .find(|s| (5..=15).contains(&s.chars().count()))
        .copied()
        .unwrap_or_default()
        .to_string();

    // 立意：正向，与文案结尾呼应
    let theme = sentences.last().copied().unwrap_or_default().to_string();

    Packaging {
        title,
        tagline,
        theme,
    }
}

// ---------------------------------------------------------------------------
// 内部：三步提炼的各步（LLM 首选，规则降级显式跟在后面）
// ---------------------------------------------------------------------------

/// 第一步：取样——从情绪记录挑含具体细节的句子（判据在 `observation.yaml`，LLM 首选）。
///
/// 判断失败取首个非空行——机械降级，不用动词表近似「具体细节」。
fn take_sample(engine: &Engine, content: &str) -> String {
    let lines: Vec<&str> = content
        .split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let Some(first) = lines.first() else {
        return content.trim().to_string();
    };

    let criteria = format!(
        "取样判据：{}。\n\ndecision 输出被选中的原句本身。",
        OBSERVATION.criteria_instruction().unwrap_or_default()
    );
    let items: Vec<Value> = lines
        .iter()
        .map(|line| Value::String((*line).to_string()))
        .collect();
    match engine.judge(&criteria, &items) {
        // 输出必须是给定句子之一，否则不采信
        Ok(judgment) if lines.contains(&judgment.decision.trim()) => {
            judgment.decision.trim().to_string()
        }
        _ => (*first).to_string(),
    }
}

/// 第二步：观察展开——从样本提取外部观察（LLM 按 `observation.yaml` 填表，首选）。
///
/// 提取具体场景、动作、细节，不是抽象感受；规则降级原样带下样本。
fn expand_observation(engine: &Engine, sample: &str) -> String {
    let doc = MarkdownDocument::parse("sample.md", sample);
    let sections = OBSERVATION.split(&doc.blocks);
    let extracted = engine
        .extract(&sections, &OBSERVATION)
        .or_else(|_| RuleBasedExtractor::new().extract(&sections, &OBSERVATION));
    match extracted {
        Ok(result) => field_text(&result.fields, "observation")
            .or(result.description)
            .unwrap_or_else(|| sample.to_string()),
        Err(_) => sample.to_string(),
    }
}

/// 第三步：落实片段——观察 → 场景素材 + 母题卡片（LLM 按 `fragment.yaml` 填表，首选）。
///
/// 规则降级：母题取来源素材标题，场景取观察截 50 字。
fn to_fragment(engine: &Engine, observation: &str, source_title: &str) -> ExtractedFragment {
    let fallback = ExtractedFragment {
        motif: source_title.to_string(),
        scene: truncate(observation, 50),
        source: source_title.to_string(),
    };
    let doc = MarkdownDocument::parse("observation.md", observation);
    let sections = FRAGMENT.split(&doc.blocks);
    let Ok(result) = engine.extract(&sections, &FRAGMENT) else {
        return fallback;
    };
    let motif = field_text(&result.fields, "motif");
    let scene = field_text(&result.fields, "scene");
    if motif.is_none() && scene.is_none() {
        return fallback;
    }
    ExtractedFragment {
        motif: motif.unwrap_or(fallback.motif),
        scene: scene
            .map(|scene| truncate(&scene, 50))
            .unwrap_or(fallback.scene),
        source: source_title.to_string(),
    }
}

/// 提取结果里的字符串字段。
fn field_text(fields: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    fields
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|text| !text.is_empty())
}

/// 正文章节已用编号（排除前言与未编号）。
fn used_numbers(novel: &Novel) -> HashSet<u32> {
    novel
        .all_chapters()
        .iter()
        .filter(|c| c.number.is_some() && !c.is_preface())
        .filter_map(|c| c.number)
        .collect()
}

/// 已用编号在 1..=max 之间的空号。
fn gaps_of(used: &HashSet<u32>) -> Vec<u32> {
    let Some(&max) = used.iter().max() else {
        return Vec::new();
    };
    (1..=max).filter(|n| !used.contains(n)).collect()
}

/// 按字符截断（50 字钩子），超长加省略号。
fn truncate(text: &str, width: usize) -> String {
    if text.chars().count() > width {
        let mut clipped: String = text.chars().take(width).collect();
        clipped.push('…');
        clipped
    } else {
        text.to_string()
    }
}
