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
//! 语义判断（分类、分级、聚类、合并、升降级）都交给 `Engine::judge`，判据是
//! [`crate::memory::rules`] 里的共享 YAML——LLM 首选、规则降级，降级路径显式写出。
//! 状态机的上下文是 [`Engine`]：判断随上下文进场，转移只消费判断结果。

use std::fmt::Write as _;

use serde_json::{Value, json};
use statig::prelude::*;

use crate::core::engine::Engine;
use crate::core::rules::Artifact;
use crate::memory::models::{InsightGrade, InsightItem, JournalEntry};
use crate::memory::rules::{CLUSTER, GRADE, INSIGHT, PROFILE, ROADMAP, categories};

/// 没有匹配的默认去向：留在原地，不是类别。
pub const JOURNAL: &str = "journal";

// ---------------------------------------------------------------------------
// 状态机：四步流程
// ---------------------------------------------------------------------------

/// memory 四步流程的状态机（无共享存储，流程数据都在状态里；判断走上下文的 Engine）。
pub struct MemoryFlow;

/// 状态机的事件：谁触发了流程、带什么进场。
#[derive(Debug)]
pub enum Event {
    /// 扫描：收集日志里的内容行（结构行不收），去向交给下一步判断。
    Scan(Vec<JournalEntry>),
    /// 分类：文本直接进场，跳过扫描。
    Classify(Vec<String>),
    /// 路由：按类别判据判断去向（作用于已扫描的候选）。
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
    fn idle(context: &mut Engine, event: &Event) -> Outcome<MemoryState> {
        match event {
            Event::Scan(entries) => Transition(MemoryState::located(scan_entries(entries))),
            Event::Classify(texts) => Transition(MemoryState::routed(classify_all(context, texts))),
            // 还没扫描/没路由，来早了
            Event::Route | Event::Cluster => Handled,
        }
    }

    // 状态局部存储的持有者必须是 &mut Vec/&mut String，不能换成切片
    #[allow(clippy::ptr_arg)]
    #[state]
    fn located(
        candidates: &mut Vec<String>,
        context: &mut Engine,
        event: &Event,
    ) -> Outcome<MemoryState> {
        match event {
            Event::Route => Transition(MemoryState::routed(classify_all(context, candidates))),
            Event::Scan(entries) => Transition(MemoryState::located(scan_entries(entries))),
            Event::Classify(texts) => Transition(MemoryState::routed(classify_all(context, texts))),
            // 没路由就不聚类
            Event::Cluster => Handled,
        }
    }

    // 状态局部存储的持有者必须是 &mut Vec/&mut String，不能换成切片
    #[allow(clippy::ptr_arg)]
    #[state]
    fn routed(
        classified: &mut Vec<ClassifiedEntry>,
        context: &mut Engine,
        event: &Event,
    ) -> Outcome<MemoryState> {
        match event {
            Event::Cluster => Transition(MemoryState::clustered(group_by_destination(classified))),
            Event::Scan(entries) => Transition(MemoryState::located(scan_entries(entries))),
            Event::Classify(texts) => Transition(MemoryState::routed(classify_all(context, texts))),
            // 已路由，不重复
            Event::Route => Handled,
        }
    }

    // 状态局部存储的持有者必须是 &mut Vec/&mut String，不能换成切片
    #[allow(clippy::ptr_arg)]
    #[state]
    fn clustered(
        classified: &mut Vec<ClassifiedEntry>,
        context: &mut Engine,
        event: &Event,
    ) -> Outcome<MemoryState> {
        match event {
            Event::Scan(entries) => Transition(MemoryState::located(scan_entries(entries))),
            Event::Classify(texts) => Transition(MemoryState::routed(classify_all(context, texts))),
            // 已到终点，状态原地不动
            Event::Route | Event::Cluster => Transition(MemoryState::clustered(classified.clone())),
        }
    }
}

impl MemoryFlow {
    /// 四步流程一步到位：扫描 → 路由 → 聚类（对照 Dart 的 `processJournal`）。
    ///
    /// Dart 侧收一个未参与计算的 `Workflow` 参数，这里不收——流程定义就是状态机本身；
    /// 语义判断需要引擎，判断器随上下文进场。
    pub fn process_journal(engine: &mut Engine, entries: &[JournalEntry]) -> Vec<ClassifiedEntry> {
        let mut machine = MemoryFlow.state_machine();
        machine.handle_with_context(&Event::Scan(entries.to_vec()), engine);
        machine.handle_with_context(&Event::Route, engine);
        machine.handle_with_context(&Event::Cluster, engine);
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

/// 分类结果：一条日志片段的去向与理由。
#[derive(Debug, Clone)]
pub struct ClassifiedEntry {
    pub text: String,
    /// 去向：命中的 Artifact 名；`None` = 没命中 = 留在 journal（补集）。
    pub destination: Option<String>,
    /// 判断理由——必填，LLM 给出；判断失败时写失败原因，不让「缺失」看起来正常。
    pub reason: String,
}

impl ClassifiedEntry {
    /// 去向名（展示与计数用），未命中记 `journal`。
    pub fn destination_name(&self) -> &str {
        self.destination.as_deref().unwrap_or(JOURNAL)
    }
}

/// 分级结果。
#[derive(Debug, Clone)]
pub struct GradedInsight {
    pub item: InsightItem,
    pub grade: InsightGrade,
}

/// 合并策略的类别：名字来自 `workflows/cluster.yaml` 的 `actions`（数据，不是编译期枚举）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeAction(String);

impl MergeAction {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// 与 YAML 里的叫法一致。
    pub fn as_str(&self) -> &str {
        &self.0
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
// 判断规则：语义判断交给 Engine，判据来自共享 YAML；降级路径显式写出
// ---------------------------------------------------------------------------

/// 分类：一次判断同时回答「归哪个类别、为什么」。
///
/// 类别与判据来自 Artifact（数据）——`route` 里再也写不出 `text.contains("要")`，
/// 因为判据不在代码里。都不命中、拿不准或判断失败 → 留在 journal，理由写进 `reason`。
pub fn classify(engine: &Engine, text: &str, categories: &[Artifact]) -> ClassifiedEntry {
    let criteria = classify_criteria(categories);
    match engine.judge(&criteria, &[Value::String(text.to_string())]) {
        Ok(judgment) => {
            let matched = categories
                .iter()
                .find(|artifact| !artifact.name.is_empty() && artifact.name == judgment.decision);
            match matched {
                Some(artifact) => ClassifiedEntry {
                    text: text.to_string(),
                    destination: Some(artifact.name.clone()),
                    reason: judgment.reason,
                },
                None if judgment.decision == JOURNAL => ClassifiedEntry {
                    text: text.to_string(),
                    destination: None,
                    reason: judgment.reason,
                },
                None => ClassifiedEntry {
                    text: text.to_string(),
                    destination: None,
                    reason: format!(
                        "无命中「{}」，留在 journal：{}",
                        judgment.decision, judgment.reason
                    ),
                },
            }
        }
        Err(err) => ClassifiedEntry {
            text: text.to_string(),
            destination: None,
            reason: format!("判断失败，留在 journal：{err}"),
        },
    }
}

/// 在册类别的判据块 + journal 补集 + 决策表，交由 `Engine::judge` 执行。
fn classify_criteria(categories: &[Artifact]) -> String {
    let mut buf = String::from("类别判据——判断文本归哪个类别，decision 输出类别名。\n");
    let mut names: Vec<&str> = Vec::new();
    for artifact in categories
        .iter()
        .filter(|artifact| !artifact.name.is_empty())
    {
        let _ = writeln!(buf, "\n### {}\n", artifact.name);
        match artifact.criteria_instruction() {
            Some(criteria) => {
                buf.push_str(&criteria);
                buf.push('\n');
            }
            None => buf.push_str("（无判据）\n"),
        }
        names.push(&artifact.name);
    }
    let _ = write!(
        buf,
        "\n### {JOURNAL}\n都不匹配或拿不准 = 留在 {JOURNAL}，decision 输出 {JOURNAL}。\n"
    );
    names.push(JOURNAL);
    let _ = write!(buf, "\ndecision 只能取：{}", names.join("、"));
    buf
}

/// 批量分类的薄包装——单条判断是概念本身，这只是状态机里的循环，不构成第二层概念。
fn classify_all(engine: &Engine, texts: &[String]) -> Vec<ClassifiedEntry> {
    // 类别是数据：从内置规则克隆一份，批量判断共用一次
    let owned: Vec<Artifact> = categories();
    texts
        .iter()
        .map(|text| classify(engine, text, &owned))
        .collect()
}

/// 证据分级：证据是输入，级别是对证据的判断（LLM 首选）。
///
/// 判据在 `workflows/grade.yaml`，决策取它的 `output`（数据）；
/// LLM 不可用时走规则降级 [`grade_by_rules`]。
pub fn grade(
    engine: &Engine,
    text: &str,
    occurrence_count: usize,
    has_verification: bool,
    is_refuted: bool,
) -> InsightGrade {
    let criteria = format!(
        "{}\n\ndecision 只能取：{}",
        GRADE.rules,
        GRADE.output.join("、")
    );
    let evidence = [json!({
        "text": text,
        "occurrence_count": occurrence_count,
        "has_verification": has_verification,
        "is_refuted": is_refuted,
    })];
    match engine.judge(&criteria, &evidence) {
        Ok(judgment) if GRADE.output.contains(&judgment.decision) => {
            InsightGrade::new(judgment.decision)
        }
        // 判断失败或决策不在册 → 规则降级
        _ => grade_by_rules(occurrence_count, has_verification, is_refuted),
    }
}

/// 规则降级：LLM 不可用时按计数判——多日重复或有验证升级，被推翻或单次降为保守档。
///
/// 升级档取 insight 规则 `grades` 的首项、保守档取末项（顺序约定写在 YAML 的 `note` 里）。
pub fn grade_by_rules(
    occurrence_count: usize,
    has_verification: bool,
    is_refuted: bool,
) -> InsightGrade {
    let Some(upgrade) = INSIGHT.grades.first() else {
        return InsightGrade::new("ungraded");
    };
    let conservative = INSIGHT.grades.last().unwrap_or(upgrade);
    if !is_refuted && (occurrence_count >= 2 || has_verification) {
        InsightGrade::new(&upgrade.name)
    } else {
        InsightGrade::new(&conservative.name)
    }
}

/// 同一认识跨日期合并：判断依据是本质相同而非措辞相似。
///
/// LLM 判断（判据在 `workflows/cluster.yaml`）；失败保守按「不同」处理——不合并不写坏数据。
pub fn cluster(engine: &Engine, items: &[InsightItem]) -> Vec<InsightItem> {
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
            if is_same_essence(engine, &current, &items[j]) {
                current = merge_insight_items(&current, &items[j]);
                used[j] = true;
            }
        }
        result.push(current);
        used[i] = true;
    }
    result
}

/// 判断命题是否本质相同（LLM）；判断失败按「不同」处理。
pub fn is_same_essence(engine: &Engine, a: &InsightItem, b: &InsightItem) -> bool {
    let criteria = format!("{}\n\ndecision 只能取：same、different", CLUSTER.rules);
    matches!(
        engine.judge(&criteria, &[item_json(a), item_json(b)]),
        Ok(judgment) if judgment.decision == "same"
    )
}

/// 判断新条目是否推翻旧条目（LLM）；判断失败按「未推翻」处理——替换是破坏性操作，不做没把握的。
pub fn is_refuted(engine: &Engine, existing: &InsightItem, incoming: &InsightItem) -> bool {
    let criteria = format!("{}\n\ndecision 只能取：refuted、not_refuted", CLUSTER.rules);
    matches!(
        engine.judge(&criteria, &[item_json(existing), item_json(incoming)]),
        Ok(judgment) if judgment.decision == "refuted"
    )
}

/// 合并策略：四种情况四种处理——新增 / 就地改写 / 被推翻替换 / 重复留互链。
///
/// 动作名来自 `workflows/cluster.yaml` 的 `actions`（首项新增、末项兜底，顺序约定在 YAML 里）。
pub fn decide_merge(
    engine: &Engine,
    existing: Option<&InsightItem>,
    incoming: InsightItem,
) -> MergeDecision {
    let Some(existing) = existing else {
        // 没有已有条目：机械新增，不用判断
        return MergeDecision {
            action: named_action(
                CLUSTER
                    .actions
                    .first()
                    .map(|action| action.name.as_str())
                    .unwrap_or("add"),
            ),
            target: incoming,
            source: None,
        };
    };
    if is_refuted(engine, existing, &incoming) {
        return MergeDecision {
            action: named_action("replace"),
            target: incoming,
            source: Some(existing.clone()),
        };
    }
    if is_same_essence(engine, existing, &incoming) {
        return MergeDecision {
            action: named_action("rewrite"),
            target: merge_insight_items(existing, &incoming),
            source: Some(existing.clone()),
        };
    }
    // 不同本质、或判断失败：留互链，不重复收录
    MergeDecision {
        action: fallback_action(),
        target: existing.clone(),
        source: Some(incoming),
    }
}

/// 动作名：在册用在册名，不在册退回内置语义名（语义绑定在代码，名单在数据）。
fn named_action(name: &str) -> MergeAction {
    CLUSTER
        .actions
        .iter()
        .find(|action| action.name == name)
        .map(|action| MergeAction::new(&action.name))
        .unwrap_or_else(|| MergeAction::new(name))
}

/// 判断失败的兜底动作：`actions` 末项（非破坏性的互链）。
fn fallback_action() -> MergeAction {
    CLUSTER
        .actions
        .last()
        .map(|action| MergeAction::new(&action.name))
        .unwrap_or_else(|| MergeAction::new("crossLink"))
}

/// 批量合并到现有列表。
pub fn merge(
    engine: &Engine,
    existing: Vec<InsightItem>,
    incoming: Vec<InsightItem>,
) -> Vec<InsightItem> {
    let mut result = existing;
    for item in incoming {
        // 找同本质的已有条目
        let matched = result
            .iter()
            .position(|e| is_same_essence(engine, e, &item));
        let existing_ref = matched.map(|index| result[index].clone());
        let decision = decide_merge(engine, existing_ref.as_ref(), item);
        match decision.action.as_str() {
            "add" => result.push(decision.target),
            "rewrite" | "replace" => {
                if let Some(index) = matched {
                    result[index] = decision.target;
                }
            }
            // 互链及未知动作：重复的留互链，不重复收录
            _ => {}
        }
    }
    result
}

/// 是否应晋升到 profile（命题反复套用、稳定为思维框架）。
///
/// 「稳定为思维框架」是 category 判断，交给 LLM；套用次数只是证据。
/// 判断失败走规则降级：反复套用（≥3 次）视为晋升。
pub fn should_promote_to_profile(
    engine: &Engine,
    item: &InsightItem,
    application_count: usize,
) -> bool {
    let criteria = format!(
        "晋升判据：命题反复套用、稳定为思维框架。\n{}\n已套用次数（证据）：{application_count}\n\ndecision 只能取：{}、keep",
        PROFILE.criteria_instruction().unwrap_or_default(),
        PROFILE.name
    );
    match engine.judge(&criteria, &[item_json(item)]) {
        Ok(judgment) => judgment.decision == PROFILE.name,
        Err(_) => application_count >= 3,
    }
}

/// 是否应分流到 roadmap（命题引出选择）。
///
/// 判断失败不分流——分流是改道动作，没把握不做。
pub fn should_fork_to_roadmap(engine: &Engine, item: &InsightItem) -> bool {
    let criteria = format!(
        "分流判据：命题引出未做的取舍。\n{}\n\ndecision 只能取：{}、keep",
        ROADMAP.criteria_instruction().unwrap_or_default(),
        ROADMAP.name
    );
    match engine.judge(&criteria, &[item_json(item)]) {
        Ok(judgment) => judgment.decision == ROADMAP.name,
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// 内部：流程各步与判断工具
// ---------------------------------------------------------------------------

/// 第一步：扫描——收集日志里的内容行。
///
/// 关键词表已删：扫描不再按关键词预筛（预筛会把「拿不准」的行提前丢掉），
/// 标题等结构行不算内容；去向由下一步的语义判断决定。
fn scan_entries(entries: &[JournalEntry]) -> Vec<String> {
    let mut candidates = Vec::new();
    for entry in entries {
        for segment in entry.segments() {
            for line in segment.split('\n') {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                candidates.push(trimmed.to_string());
            }
        }
    }
    candidates
}

/// 第三步：聚类——按去向分组，同组内聚类。
///
/// 简化：不跨条目聚类，由上层用 [`cluster`] 处理。
fn group_by_destination(entries: &[ClassifiedEntry]) -> Vec<ClassifiedEntry> {
    let mut groups: Vec<(Option<String>, Vec<ClassifiedEntry>)> = Vec::new();
    for entry in entries {
        match groups
            .iter_mut()
            .find(|(destination, _)| *destination == entry.destination)
        {
            Some((_, group)) => group.push(entry.clone()),
            None => groups.push((entry.destination.clone(), vec![entry.clone()])),
        }
    }
    groups.into_iter().flat_map(|(_, group)| group).collect()
}

/// 条目 → 判断输入。
fn item_json(item: &InsightItem) -> Value {
    json!({
        "statement": item.statement,
        "detail": item.detail,
        "evidence": item.evidence,
    })
}

/// 两条洞察合并为一条：命题留首条，解释合并，证据取首个非空。
fn merge_insight_items(a: &InsightItem, b: &InsightItem) -> InsightItem {
    InsightItem {
        statement: a.statement.clone(),
        detail: format!("{}；{}", a.detail, b.detail),
        evidence: a.evidence.clone().or_else(|| b.evidence.clone()),
    }
}
