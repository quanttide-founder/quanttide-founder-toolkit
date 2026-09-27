//! memory/states：四步流程的状态机（statig）+ 判断规则。
//!
//! 事件进 → 状态出：
//!
//! ```text
//! Idle ──Scan──▶ Located ──Route──▶ Routed ──Cluster──▶ Clustered
//!   └────────Classify─────────▶ Routed
//! ```
//!
//! 扫描、路由、聚类每步只做一件事，结果放进当前状态（state-local storage）；
//! 事件没到、或走错了步（路由时还没扫描），状态机不理会——`Handled`。
//!
//! 三问路由、证据分级、合并策略、升降级是纯判断，不随状态走，收成本模块的自由函数。
//! 对照 Dart 包的 `memory/bloc.dart`：bloc 换成 statig 状态机，规则逐条对齐。

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;
use statig::prelude::*;

use crate::memory::models::{InsightGrade, InsightItem, JournalEntry};

// ---------------------------------------------------------------------------
// 状态机：四步流程
// ---------------------------------------------------------------------------

/// memory 四步流程的状态机（无共享存储，流程数据都在状态里）。
pub struct MemoryFlow;

/// 状态机的事件：谁触发了流程、带什么进场。
#[derive(Debug)]
pub enum Event {
    /// 扫描：从日志条目按关键词定位候选。
    Scan(Vec<JournalEntry>),
    /// 分类：文本直接进场，跳过扫描。
    Classify(Vec<String>),
    /// 路由：三问判断去向（作用于已扫描的候选）。
    Route,
    /// 聚类：按去向分组（作用于已路由的条目）。
    Cluster,
}

#[state_machine(
    initial = "MemoryState::idle()",
    state(name = "MemoryState", derive(Debug))
)]
impl MemoryFlow {
    #[state]
    fn idle(event: &Event) -> Outcome<MemoryState> {
        match event {
            Event::Scan(entries) => Transition(MemoryState::located(scan_entries(entries))),
            Event::Classify(texts) => Transition(MemoryState::routed(classify(texts))),
            // 还没扫描/没路由，来早了
            Event::Route | Event::Cluster => Handled,
        }
    }

    // 状态局部存储的持有者必须是 &mut Vec/&mut String，不能换成切片
    #[allow(clippy::ptr_arg)]
    #[state]
    fn located(candidates: &mut Vec<String>, event: &Event) -> Outcome<MemoryState> {
        match event {
            Event::Route => Transition(MemoryState::routed(classify(candidates))),
            Event::Scan(entries) => Transition(MemoryState::located(scan_entries(entries))),
            Event::Classify(texts) => Transition(MemoryState::routed(classify(texts))),
            // 没路由就不聚类
            Event::Cluster => Handled,
        }
    }

    // 状态局部存储的持有者必须是 &mut Vec/&mut String，不能换成切片
    #[allow(clippy::ptr_arg)]
    #[state]
    fn routed(classified: &mut Vec<ClassifiedEntry>, event: &Event) -> Outcome<MemoryState> {
        match event {
            Event::Cluster => Transition(MemoryState::clustered(group_by_destination(classified))),
            Event::Scan(entries) => Transition(MemoryState::located(scan_entries(entries))),
            Event::Classify(texts) => Transition(MemoryState::routed(classify(texts))),
            // 已路由，不重复
            Event::Route => Handled,
        }
    }

    // 状态局部存储的持有者必须是 &mut Vec/&mut String，不能换成切片
    #[allow(clippy::ptr_arg)]
    #[state]
    fn clustered(classified: &mut Vec<ClassifiedEntry>, event: &Event) -> Outcome<MemoryState> {
        match event {
            Event::Scan(entries) => Transition(MemoryState::located(scan_entries(entries))),
            Event::Classify(texts) => Transition(MemoryState::routed(classify(texts))),
            // 已到终点，状态原地不动
            Event::Route | Event::Cluster => Transition(MemoryState::clustered(classified.clone())),
        }
    }
}

impl MemoryFlow {
    /// 四步流程一步到位：扫描 → 路由 → 聚类（对照 Dart 的 `processJournal`）。
    ///
    /// Dart 侧收一个未参与计算的 `Workflow` 参数，这里不收——流程定义就是状态机本身。
    pub fn process_journal(entries: &[JournalEntry]) -> Vec<ClassifiedEntry> {
        let mut machine = MemoryFlow.state_machine();
        machine.handle(&Event::Scan(entries.to_vec()));
        machine.handle(&Event::Route);
        machine.handle(&Event::Cluster);
        machine
            .state()
            .classified()
            .map(<[ClassifiedEntry]>::to_vec)
            .unwrap_or_default()
    }
}

impl MemoryState {
    /// 当前状态携带的候选（仅 Located）。
    pub fn candidates(&self) -> Option<&[String]> {
        match self {
            MemoryState::Located { candidates } => Some(candidates),
            _ => None,
        }
    }

    /// 当前状态携带的分类结果（Routed 或 Clustered）。
    pub fn classified(&self) -> Option<&[ClassifiedEntry]> {
        match self {
            MemoryState::Routed { classified } | MemoryState::Clustered { classified } => {
                Some(classified)
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// 去向与结果类型
// ---------------------------------------------------------------------------

/// 去向：三问路由的落点。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Destination {
    Insight,
    Profile,
    Roadmap,
    Intention,
    Journal,
}

impl Destination {
    /// 与 YAML / 文档里的叫法一致。
    pub fn as_str(self) -> &'static str {
        match self {
            Destination::Insight => "insight",
            Destination::Profile => "profile",
            Destination::Roadmap => "roadmap",
            Destination::Intention => "intention",
            Destination::Journal => "journal",
        }
    }
}

/// 分类结果：一条日志片段的去向。
#[derive(Debug, Clone)]
pub struct ClassifiedEntry {
    pub text: String,
    pub destination: Destination,
    pub reason: Option<String>,
}

/// 分级结果。
#[derive(Debug, Clone)]
pub struct GradedInsight {
    pub item: InsightItem,
    pub grade: InsightGrade,
}

/// 合并策略的四种情况。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeAction {
    Add,
    Rewrite,
    Replace,
    CrossLink,
}

impl MergeAction {
    /// 与文档里的叫法一致。
    pub fn as_str(self) -> &'static str {
        match self {
            MergeAction::Add => "add",
            MergeAction::Rewrite => "rewrite",
            MergeAction::Replace => "replace",
            MergeAction::CrossLink => "crossLink",
        }
    }
}

/// 合并决策。
#[derive(Debug, Clone)]
pub struct MergeDecision {
    pub action: MergeAction,
    pub target: InsightItem,
    pub source: Option<InsightItem>,
}

// ---------------------------------------------------------------------------
// 判断规则：纯函数，不随状态走
// ---------------------------------------------------------------------------

/// 三问过滤：判断一条文本的去向。
///
/// 「想通了什么」→ insight、「我是什么样的人」→ profile、
/// 「我要做什么」→ roadmap、「我到底想要什么」→ intention、
/// 「发生了什么」→ 留 journal。
pub fn route(text: &str) -> Destination {
    // 意图类关键词优先（要/不要/别/澄清），因为意图是最明确的指令
    if matches_any(text, &INTENT_KEYWORDS) {
        return Destination::Intention;
    }
    // 方向类关键词（决定/优先/待定）
    if matches_any(text, &DIRECTION_KEYWORDS) {
        return Destination::Roadmap;
    }
    // 认知类关键词（发现/原来/其实/关键是）
    if matches_any(text, &COGNITION_KEYWORDS) {
        return Destination::Insight;
    }
    // 特征类关键词（我总是/我习惯/我害怕）
    if matches_any(text, &PROFILE_KEYWORDS) {
        return Destination::Profile;
    }
    // 默认留 journal
    Destination::Journal
}

/// 批量分类。
pub fn classify(texts: &[String]) -> Vec<ClassifiedEntry> {
    texts
        .iter()
        .map(|text| ClassifiedEntry {
            text: text.clone(),
            destination: route(text),
            reason: None,
        })
        .collect()
}

/// 证据分级：多日重复或有验证 → 已确认；单次 → 假说；被推翻 → 移除。
///
/// 与 Dart 侧同签名保留 `text`（暂未参与判定，只用三个计数）。
pub fn grade(
    _text: &str,
    occurrence_count: usize,
    has_verification: bool,
    is_refuted: bool,
) -> InsightGrade {
    if is_refuted {
        // 被推翻的降级，由合并阶段移除
        return InsightGrade::Hypothesis;
    }
    if occurrence_count >= 2 || has_verification {
        return InsightGrade::Confirmed;
    }
    InsightGrade::Hypothesis
}

/// 同一认识跨日期合并：判断依据是本质相同而非措辞相似。
///
/// 简化实现：按关键词重合度判断，实际由 LLM 执行（`Engine::judge`）。
pub fn cluster(items: &[InsightItem]) -> Vec<InsightItem> {
    let mut result: Vec<InsightItem> = Vec::new();
    let mut used = vec![false; items.len()];

    for i in 0..items.len() {
        if used[i] {
            continue;
        }
        let mut current = items[i].clone();
        for j in i + 1..items.len() {
            if used[j] {
                continue;
            }
            // 本质相同 → 合并；不同 → 各自保留
            if is_same_essence(&current, &items[j]) {
                current = merge_insight_items(&current, &items[j]);
                used[j] = true;
            }
        }
        result.push(current);
        used[i] = true;
    }
    result
}

/// 合并策略：四种情况四种处理——新增 / 就地改写 / 被推翻替换 / 重复留互链。
pub fn decide_merge(existing: Option<&InsightItem>, incoming: InsightItem) -> MergeDecision {
    let Some(existing) = existing else {
        return MergeDecision {
            action: MergeAction::Add,
            target: incoming,
            source: None,
        };
    };
    if is_refuted(existing, &incoming) {
        return MergeDecision {
            action: MergeAction::Replace,
            target: incoming,
            source: Some(existing.clone()),
        };
    }
    if is_same_essence(existing, &incoming) {
        return MergeDecision {
            action: MergeAction::Rewrite,
            target: merge_insight_items(existing, &incoming),
            source: Some(existing.clone()),
        };
    }
    MergeDecision {
        action: MergeAction::CrossLink,
        target: existing.clone(),
        source: Some(incoming),
    }
}

/// 批量合并到现有列表。
pub fn merge(existing: Vec<InsightItem>, incoming: Vec<InsightItem>) -> Vec<InsightItem> {
    let mut result = existing;
    for item in incoming {
        // 找同本质的已有条目
        let matched = result.iter().position(|e| is_same_essence(e, &item));
        let existing_ref = matched.map(|index| result[index].clone());
        let decision = decide_merge(existing_ref.as_ref(), item);
        match decision.action {
            MergeAction::Add => result.push(decision.target),
            MergeAction::Rewrite | MergeAction::Replace => {
                if let Some(index) = matched {
                    result[index] = decision.target;
                }
            }
            MergeAction::CrossLink => {} // 重复的留互链，不重复收录
        }
    }
    result
}

/// 是否应晋升到 profile（命题反复套用、稳定为思维框架）。
///
/// 与 Dart 侧同签名保留 `item`（只看套用次数）。
pub fn should_promote_to_profile(_item: &InsightItem, application_count: usize) -> bool {
    application_count >= 3 // 反复套用 = 晋升
}

/// 是否应分流到 roadmap（命题引出选择）。
pub fn should_fork_to_roadmap(item: &InsightItem) -> bool {
    ["应该", "需要", "决定"]
        .iter()
        .any(|kw| item.detail.contains(kw))
}

// ---------------------------------------------------------------------------
// 内部：流程各步与判断工具
// ---------------------------------------------------------------------------

/// 第一步：扫描——按关键词定位候选。
fn scan_entries(entries: &[JournalEntry]) -> Vec<String> {
    let mut candidates = Vec::new();
    for entry in entries {
        for segment in entry.segments() {
            for line in segment.split('\n') {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if matches_any(trimmed, &ALL_KEYWORDS) {
                    candidates.push(trimmed.to_string());
                }
            }
        }
    }
    candidates
}

/// 第三步：聚类——按去向分组，同组内聚类。
///
/// 简化：不跨条目聚类，由上层用 [`cluster`] 处理。
fn group_by_destination(entries: &[ClassifiedEntry]) -> Vec<ClassifiedEntry> {
    let mut groups: Vec<(Destination, Vec<ClassifiedEntry>)> = Vec::new();
    for entry in entries {
        match groups
            .iter_mut()
            .find(|(dest, _)| *dest == entry.destination)
        {
            Some((_, group)) => group.push(entry.clone()),
            None => groups.push((entry.destination, vec![entry.clone()])),
        }
    }
    groups.into_iter().flat_map(|(_, group)| group).collect()
}

fn matches_any(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|kw| text.contains(kw))
}

/// 判断命题是否本质相同——实际由 LLM 判断，这里用关键词重合度做粗筛。
fn is_same_essence(a: &InsightItem, b: &InsightItem) -> bool {
    let a_words: HashSet<&str> = WORD_SPLIT
        .split(a.statement.as_str())
        .filter(|w| w.chars().count() >= 2)
        .collect();
    let b_words: HashSet<&str> = WORD_SPLIT
        .split(b.statement.as_str())
        .filter(|w| w.chars().count() >= 2)
        .collect();
    if a_words.is_empty() || b_words.is_empty() {
        return false;
    }
    let overlap = a_words.intersection(&b_words).count();
    overlap as f64 / a_words.len() as f64 >= 0.5
}

/// 被推翻 = 新条目明确否定旧条目。
fn is_refuted(existing: &InsightItem, incoming: &InsightItem) -> bool {
    if !incoming.statement.contains("不是") {
        return false;
    }
    let negated = incoming.statement.replace("不是", "");
    existing.statement.contains(negated.trim())
}

fn merge_insight_items(a: &InsightItem, b: &InsightItem) -> InsightItem {
    InsightItem {
        statement: a.statement.clone(),
        detail: format!("{}；{}", a.detail, b.detail),
        evidence: a.evidence.clone().or_else(|| b.evidence.clone()),
    }
}

// ---------------------------------------------------------------------------
// 关键词表（来自 doc/memory.md）
// ---------------------------------------------------------------------------

/// 命题分词：按全角标点与空白切。
static WORD_SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[，。、\s]+").unwrap());

const COGNITION_KEYWORDS: [&str; 9] = [
    "发现",
    "意识",
    "原来",
    "其实",
    "关键是",
    "临界点",
    "原因在于",
    "实际上",
    "规律",
];
const INTENT_KEYWORDS: [&str; 12] = [
    "要", "不要", "别", "希望", "应该", "必须", "交给", "按", "先", "只", "统一", "澄清",
];
const DIRECTION_KEYWORDS: [&str; 9] = [
    "决定",
    "决定不",
    "先做",
    "优先",
    "不动",
    "合并",
    "拆开",
    "待定",
    "问题是",
];
const PROFILE_KEYWORDS: [&str; 5] = ["我总是", "我习惯", "我害怕", "我反复", "我如何判断"];

static ALL_KEYWORDS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    COGNITION_KEYWORDS
        .iter()
        .chain(INTENT_KEYWORDS.iter())
        .chain(DIRECTION_KEYWORDS.iter())
        .chain(PROFILE_KEYWORDS.iter())
        .copied()
        .collect()
});
