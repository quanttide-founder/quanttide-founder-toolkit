//! memory 域：仓库装载 + 四步流程状态机 + 判断规则。

mod common;

use std::collections::HashSet;

use chrono::NaiveDate;
use common::memory_fixture;
use quanttide_founder::Error;
use quanttide_founder::memory::models::{InsightGrade, InsightItem, JournalSource};
use quanttide_founder::memory::states::{
    Destination, Event, MemoryFlow, MemoryState, MergeAction, classify, cluster, decide_merge,
    grade, merge, route, should_fork_to_roadmap, should_promote_to_profile,
};
use quanttide_founder::{InsightDoc, MemoryRepository, ProfileDoc, RoadmapDoc};
use statig::prelude::*;

fn load_fixture() -> (tempfile::TempDir, MemoryRepository) {
    let dir = tempfile::tempdir().expect("建临时目录");
    memory_fixture(dir.path());
    let repo = MemoryRepository::load(dir.path()).expect("装载仓库");
    (dir, repo)
}

// ---------------------------------------------------------------------------
// 仓库装载
// ---------------------------------------------------------------------------

#[test]
fn loads_sets_layers_and_skips_non_sets() {
    let (_dir, repo) = load_fixture();
    let names: Vec<&str> = repo.sets.iter().map(|s| s.name.as_str()).collect();
    // 杂物/（无层目录）与 .hidden/（隐藏）都不算记忆集
    assert_eq!(names, vec!["default"]);
}

#[test]
fn journals_are_dated_sorted_and_filtered() {
    let (_dir, repo) = load_fixture();
    let set = &repo.sets[0];
    assert_eq!(set.journals.len(), 2);

    // 按日期倒序；集根当天在前、journal/ 归档在后
    assert_eq!(
        set.journals[0].date,
        NaiveDate::from_ymd_opt(2026, 9, 1).expect("合法日期")
    );
    assert_eq!(set.journals[0].source, JournalSource::Root);
    assert_eq!(
        set.journals[1].date,
        NaiveDate::from_ymd_opt(2026, 8, 20).expect("合法日期")
    );
    assert_eq!(set.journals[1].source, JournalSource::Archive);

    // README.md 与非日期文件不收
    assert!(set.journals.iter().all(|j| !j.path.ends_with("README.md")));
    assert!(
        set.journals
            .iter()
            .all(|j| !j.path.ends_with("not-a-date.md"))
    );

    // 全仓库时间线同样倒序
    let all = repo.all_journals();
    assert_eq!(all.len(), 2);
    assert!(all[0].date > all[1].date);
}

#[test]
fn journal_segments_and_character_count() {
    let (_dir, repo) = load_fixture();
    let root_journal = repo.sets[0]
        .journals
        .iter()
        .find(|j| j.source == JournalSource::Root)
        .expect("有集根日志");

    let segments = root_journal.segments();
    assert_eq!(segments.len(), 2); // `---` 切开
    assert!(segments[0].contains("今天发现原来的缓存策略有个漏洞"));
    assert!(segments[1].contains("我决定先做登录页"));
    assert!(segments.iter().all(|s| !s.trim().is_empty()));

    // 去掉全部空白后的字数
    assert_eq!(
        root_journal.character_count(),
        root_journal
            .content
            .chars()
            .filter(|c| !c.is_whitespace())
            .count()
    );
}

#[test]
fn profile_doc_parses_sections() {
    let (_dir, repo) = load_fixture();
    let profile: &ProfileDoc = &repo.sets[0].profiles[0];
    assert_eq!(profile.name, "profile");
    assert_eq!(profile.title.as_deref(), Some("张三"));
    assert_eq!(profile.description, "来自长期观察的档案。");
    assert_eq!(profile.sections.len(), 2);
    assert_eq!(profile.sections[0].title, "工作方式");
    assert_eq!(profile.sections[0].bullets.len(), 2);
    assert_eq!(profile.sections[1].title, "沟通");
    assert_eq!(profile.sections[1].paragraphs, vec!["说重点，不绕弯。"]);
    // README 不收
    assert_eq!(repo.sets[0].profiles.len(), 1);
}

#[test]
fn insight_doc_parses_grades_items_and_evidence() {
    let (_dir, repo) = load_fixture();
    let insight: &InsightDoc = &repo.sets[0].insights[0];
    assert_eq!(insight.title.as_deref(), Some("洞察"));
    assert_eq!(insight.sections.len(), 2);
    assert_eq!(insight.sections[0].grade, Some(InsightGrade::Confirmed));
    assert_eq!(insight.sections[1].grade, Some(InsightGrade::Hypothesis));

    let confirmed: Vec<&InsightItem> = insight.items_of(InsightGrade::Confirmed).collect();
    assert_eq!(confirmed.len(), 1);
    assert_eq!(confirmed[0].statement, "缓存失效的规律是 TTL 太长");
    // 切在「依据：」前，其前的标点留着（同 Dart：trim 只去空白）
    assert_eq!(confirmed[0].detail, "读多写少场景适用，");
    assert_eq!(confirmed[0].evidence.as_deref(), Some("连续三周复现"));

    let hypothesis: Vec<&InsightItem> = insight.items_of(InsightGrade::Hypothesis).collect();
    assert_eq!(hypothesis[0].statement, "单点登录可能是瓶颈");
    assert_eq!(hypothesis[0].evidence, None);
}

#[test]
fn insight_doc_without_h2_is_one_prose_section() {
    // 无二级标题：全文作为一节（主题式散文），标题取 H1
    let doc = InsightDoc::parse("thoughts.md", "# 随想\n\n想到就写，不拘结构。\n");
    assert_eq!(doc.sections.len(), 1);
    assert_eq!(doc.sections[0].title, "随想");
    assert_eq!(doc.sections[0].grade, None);
    assert_eq!(
        doc.sections[0].paragraphs,
        vec!("想到就写，不拘结构。".to_string())
    );
}

#[test]
fn roadmap_doc_parses_layers_and_themes() {
    let (_dir, repo) = load_fixture();
    let roadmap: &RoadmapDoc = &repo.sets[0].roadmaps[0];
    assert_eq!(roadmap.title.as_deref(), Some("路线图"));
    assert_eq!(roadmap.goal.as_deref(), Some("今年把发布流程跑顺。"));
    assert_eq!(roadmap.meta_goals.len(), 1);
    assert_eq!(roadmap.meta_goals[0].name, "自动化发布");
    assert_eq!(roadmap.core_problems.len(), 1);
    assert_eq!(roadmap.core_problems[0].name, "发布太慢");
    assert_eq!(roadmap.decided.len(), 1);
    assert_eq!(roadmap.pending.len(), 1);
    assert_eq!(roadmap.theme_sections.len(), 1);
    assert_eq!(roadmap.theme_sections[0].title, "写作");
}

#[test]
fn missing_root_reports_domain() {
    let err = MemoryRepository::load("/definitely/not/here").expect_err("根目录不存在");
    assert!(matches!(
        err,
        Error::RootNotFound {
            domain: "memory",
            ..
        }
    ));
    assert!(err.to_string().contains("memory 仓库根目录不存在"));
}

// ---------------------------------------------------------------------------
// 四步流程状态机
// ---------------------------------------------------------------------------

#[test]
fn events_walk_idle_located_routed_clustered() {
    let (_dir, repo) = load_fixture();
    let entries = repo.sets[0].journals.clone();

    let mut machine = MemoryFlow.state_machine();
    // 来早了：没扫描就路由，不理会
    machine.handle(&Event::Route);
    assert!(matches!(machine.state(), MemoryState::Idle { .. }));
    assert!(machine.state().candidates().is_none());

    machine.handle(&Event::Scan(entries));
    let candidates = machine.state().candidates().expect("已扫描").to_vec();
    // 关键词扫描只收候选（发现/我总是/先…）
    assert_eq!(
        candidates,
        vec![
            "今天发现原来的缓存策略有个漏洞".to_string(),
            "我决定先做登录页".to_string(),
            "归档：我总是把事情拖到最后".to_string(),
        ]
    );

    machine.handle(&Event::Route);
    let classified = machine.state().classified().expect("已路由").to_vec();
    assert_eq!(classified[0].destination, Destination::Insight);
    assert_eq!(classified[1].destination, Destination::Intention);
    assert_eq!(classified[2].destination, Destination::Profile);

    machine.handle(&Event::Cluster);
    let grouped = machine.state().classified().expect("已聚类").to_vec();
    assert_eq!(grouped.len(), 3); // 三个去向各一条，按去向分组

    // 已到终点：重复事件不改变状态
    machine.handle(&Event::Cluster);
    assert!(matches!(machine.state(), MemoryState::Clustered { .. }));
}

#[test]
fn classify_event_enters_pipeline_directly() {
    let texts = vec!["今天开了三个会".to_string(), "原来是排期冲突".to_string()];
    let mut machine = MemoryFlow.state_machine();
    machine.handle(&Event::Classify(texts));
    let classified = machine.state().classified().expect("已路由");
    assert_eq!(classified[0].destination, Destination::Journal);
    assert_eq!(classified[1].destination, Destination::Insight);
}

#[test]
fn process_journal_runs_all_steps() {
    let (_dir, repo) = load_fixture();
    let classified = MemoryFlow::process_journal(&repo.sets[0].journals);
    assert_eq!(classified.len(), 3);
    let destinations: HashSet<&str> = classified.iter().map(|c| c.destination.as_str()).collect();
    assert_eq!(
        destinations,
        HashSet::from(["insight", "intention", "profile"])
    );
}

// ---------------------------------------------------------------------------
// 判断规则
// ---------------------------------------------------------------------------

#[test]
fn routes_text_by_three_questions_in_priority_order() {
    // 意图类优先于方向/认知
    assert_eq!(route("先把发布流程澄清"), Destination::Intention);
    // 「不动」命中方向关键词，且不含意图词（注意「优先」含「先」，会先被判成意图）
    assert_eq!(route("决定不动这块"), Destination::Roadmap);
    assert_eq!(route("原来问题的关键是缓存"), Destination::Insight);
    assert_eq!(route("我总是把截止日当起点"), Destination::Profile);
    assert_eq!(route("今天开了三个会"), Destination::Journal);
}

#[test]
fn classifies_in_batch() {
    let texts = vec!["今天开了三个会".to_string(), "决定先做登录页".to_string()];
    let classified = classify(&texts);
    assert_eq!(classified.len(), 2);
    assert_eq!(classified[0].text, texts[0]);
    assert_eq!(classified[0].destination, Destination::Journal);
    assert_eq!(classified[1].destination, Destination::Intention); // 「先」是意图关键词
}

#[test]
fn grades_by_evidence() {
    // 单次、未验证 → 假说
    assert_eq!(grade("x", 1, false, false), InsightGrade::Hypothesis);
    // 多日重复 → 已确认
    assert_eq!(grade("x", 2, false, false), InsightGrade::Confirmed);
    // 有验证 → 已确认
    assert_eq!(grade("x", 1, true, false), InsightGrade::Confirmed);
    // 被推翻 → 降级为假说（由合并阶段移除）
    assert_eq!(grade("x", 5, true, true), InsightGrade::Hypothesis);
}

#[test]
fn clusters_by_essence_not_wording() {
    let a = InsightItem::new("缓存失效的规律是 TTL 太长", "读多写少适用");
    let b = InsightItem::new("缓存失效的规律其实是 TTL 太长", "写多读少不适用");
    let other = InsightItem::new("会议纪要要当天写完", "当天发群里");

    let clustered = cluster(&[a.clone(), b, other]);
    assert_eq!(clustered.len(), 2);
    assert_eq!(clustered[0].statement, a.statement);
    assert_eq!(clustered[0].detail, "读多写少适用；写多读少不适用");

    // 单条不动
    assert_eq!(cluster(&[a]).len(), 1);
}

#[test]
fn merge_follows_four_strategies() {
    let existing = InsightItem::new("缓存失效的规律是 TTL 太长", "读多写少适用");

    // 新线索 → 增条目
    let decision = decide_merge(None, InsightItem::new("新命题", ""));
    assert_eq!(decision.action, MergeAction::Add);
    assert_eq!(decision.target.statement, "新命题");
    assert!(decision.source.is_none());

    // 同本质 → 就地改写
    let twin = InsightItem::new("缓存失效的规律是 TTL 太长", "重复记了一次");
    let decision = decide_merge(Some(&existing), twin);
    assert_eq!(decision.action, MergeAction::Rewrite);
    assert_eq!(decision.target.detail, "读多写少适用；重复记了一次");

    // 被推翻 → 直接替换
    // 被推翻 = 新条目含「不是」，且去掉「不是」后正是旧条目
    let refuted = InsightItem::new("计划不是先做 A", "");
    let plan = InsightItem::new("计划先做 A", "");
    let decision = decide_merge(Some(&plan), refuted);
    assert_eq!(decision.action, MergeAction::Replace);
    assert_eq!(decision.target.statement, "计划不是先做 A");

    // 重复但不同本质 → 留互链，不重复收录
    let decision = decide_merge(Some(&existing), InsightItem::new("完全另一个命题", ""));
    assert_eq!(decision.action, MergeAction::CrossLink);
    assert_eq!(decision.target.statement, existing.statement);

    // 批量合并：改写不扩容、新增才扩容
    let merged = merge(
        vec![existing.clone()],
        vec![
            InsightItem::new("缓存失效的规律是 TTL 太长", "又记一次"),
            InsightItem::new("完全另一个命题", ""),
        ],
    );
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].detail, "读多写少适用；又记一次");
    assert_eq!(merged[1].statement, "完全另一个命题");
}

#[test]
fn promotion_and_fork_rules() {
    let item = InsightItem::new("命题", "应该先做最小闭环");
    assert!(!should_promote_to_profile(&item, 2));
    assert!(should_promote_to_profile(&item, 3));
    assert!(should_fork_to_roadmap(&item));
    assert!(!should_fork_to_roadmap(&InsightItem::new(
        "命题",
        "没有行动词"
    )));
}
