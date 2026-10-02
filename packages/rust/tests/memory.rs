//! memory 域：仓库装载 + 四步流程状态机 + 判断规则。
//!
//! 判断走 LLM 首选（`semantic_llm` 按判据回 JSON）；规则降级用 `demo_llm`（回纯文本，判断必然失败）。

mod common;

use std::collections::HashSet;

use chrono::NaiveDate;
use common::{demo_llm, memory_fixture, semantic_llm};
use quanttide_founder::Error;
use quanttide_founder::core::engine::Engine;
use quanttide_founder::core::rules::Artifact;
use quanttide_founder::memory::models::{InsightGrade, InsightItem, JournalSource};
use quanttide_founder::memory::rules::categories;
use quanttide_founder::memory::states::{
    Event, JOURNAL, MemoryFlow, MemoryState, classify, cluster, decide_merge, grade,
    grade_by_rules, merge, should_fork_to_roadmap, should_promote_to_profile,
};
use quanttide_founder::{InsightDoc, MemoryRepository, ProfileDoc, RoadmapDoc};
use statig::prelude::*;

fn load_fixture() -> (tempfile::TempDir, MemoryRepository) {
    let dir = tempfile::tempdir().expect("建临时目录");
    memory_fixture(dir.path());
    let repo = MemoryRepository::load(dir.path()).expect("装载仓库");
    (dir, repo)
}

fn semantic_engine() -> Engine {
    Engine::new(semantic_llm())
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
fn journal_segments_character_count_and_tags() {
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

    // tag 是元数据提取，不是语义判断：日期从文件名来，来源从路径来
    let tags = root_journal.tags();
    assert_eq!(tags[0].key, "date");
    assert_eq!(tags[0].value, "2026-09-01");
    assert_eq!(tags[1].key, "source");
    assert_eq!(tags[1].value, "root");
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
    // 分级名来自 insight.yaml 的 grades（category 是数据）
    assert_eq!(
        insight.sections[0].grade,
        Some(InsightGrade::new("confirmed"))
    );
    assert_eq!(
        insight.sections[1].grade,
        Some(InsightGrade::new("hypothesis"))
    );

    let confirmed: Vec<&InsightItem> = insight.items_of("confirmed").collect();
    assert_eq!(confirmed.len(), 1);
    assert_eq!(confirmed[0].statement, "缓存失效的规律是 TTL 太长");
    // 切在「依据：」前，其前的标点留着（同 Dart：trim 只去空白）
    assert_eq!(confirmed[0].detail, "读多写少场景适用，");
    assert_eq!(confirmed[0].evidence.as_deref(), Some("连续三周复现"));

    let hypothesis: Vec<&InsightItem> = insight.items_of("hypothesis").collect();
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
fn other_rules_reparse_the_same_document() {
    // 换一份 YAML 就换一种解读方式：改分级标题与切分层级，代码不动
    let rule = Artifact::from_yaml(
        r#"
document_type: insight
title: {source: first_h1, fallback: filename}
description: {location: preface, meaning: 说明}
sections: {split_by: H3}
grades:
  - name: stable
    titles: [稳定]
"#,
    )
    .expect("合法规则");
    let doc = InsightDoc::parse_with(
        "t.md",
        "# 洞察\n\n前言。\n\n### 稳定\n\n- **命题**：解释\n",
        &rule,
    );
    assert_eq!(doc.sections.len(), 1);
    assert_eq!(doc.sections[0].title, "稳定");
    assert_eq!(doc.sections[0].grade, Some(InsightGrade::new("stable")));
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
    let mut engine = semantic_engine();

    let mut machine = MemoryFlow.state_machine();
    // 来早了：没扫描就路由，不理会
    machine.handle_with_context(&Event::Route, &mut engine);
    assert!(matches!(machine.state(), MemoryState::Idle { .. }));
    assert!(machine.state().candidates().is_none());

    machine.handle_with_context(&Event::Scan(entries), &mut engine);
    let candidates = machine.state().candidates().expect("已扫描").to_vec();
    // 扫描只收内容行（标题行不收），去向交给分类判断
    assert_eq!(
        candidates,
        vec![
            "今天发现原来的缓存策略有个漏洞".to_string(),
            "我决定先做登录页".to_string(),
            "归档：我总是把事情拖到最后".to_string(),
        ]
    );

    machine.handle_with_context(&Event::Route, &mut engine);
    let classified = machine.state().classified().expect("已路由").to_vec();
    assert_eq!(classified[0].destination.as_deref(), Some("insight"));
    assert_eq!(classified[1].destination.as_deref(), Some("intention"));
    assert_eq!(classified[2].destination.as_deref(), Some("profile"));
    // 判断理由必填
    assert!(classified[0].reason.contains("测试判断"));

    machine.handle_with_context(&Event::Cluster, &mut engine);
    let grouped = machine.state().classified().expect("已聚类").to_vec();
    assert_eq!(grouped.len(), 3); // 三个去向各一条，按去向分组

    // 已到终点：重复事件不改变状态
    machine.handle_with_context(&Event::Cluster, &mut engine);
    assert!(matches!(machine.state(), MemoryState::Clustered { .. }));
}

#[test]
fn classify_event_enters_pipeline_directly() {
    let texts = vec!["今天开了三个会".to_string(), "原来是排期冲突".to_string()];
    let mut engine = semantic_engine();
    let mut machine = MemoryFlow.state_machine();
    machine.handle_with_context(&Event::Classify(texts), &mut engine);
    let classified = machine.state().classified().expect("已路由");
    assert_eq!(classified[0].destination, None);
    assert_eq!(classified[0].destination_name(), JOURNAL);
    assert_eq!(classified[1].destination.as_deref(), Some("insight"));
}

#[test]
fn process_journal_runs_all_steps() {
    let (_dir, repo) = load_fixture();
    let mut engine = semantic_engine();
    let classified = MemoryFlow::process_journal(&mut engine, &repo.sets[0].journals);
    assert_eq!(classified.len(), 3);
    let destinations: HashSet<&str> = classified.iter().map(|c| c.destination_name()).collect();
    assert_eq!(
        destinations,
        HashSet::from(["insight", "intention", "profile"])
    );
}

// ---------------------------------------------------------------------------
// 判断规则
// ---------------------------------------------------------------------------

#[test]
fn classify_judges_by_criteria_and_keeps_journal_as_complement() {
    let engine = semantic_engine();
    let categories = categories();

    // 判据来自 YAML，类别来自 Artifact 的 name
    let entry = classify(&engine, "原来问题的关键是缓存", &categories);
    assert_eq!(entry.destination.as_deref(), Some("insight"));
    assert!(!entry.reason.is_empty());

    // 都不命中 → journal（补集，不是类别）
    let entry = classify(&engine, "今天开了三个会", &categories);
    assert_eq!(entry.destination, None);
    assert_eq!(entry.destination_name(), JOURNAL);

    // 决策不在册 → journal，理由写明无命中
    let entry = classify(&engine, "乱码这句话", &categories);
    assert_eq!(entry.destination, None);
    assert!(entry.reason.contains("无命中"));
}

#[test]
fn classify_follows_supplied_categories_not_code() {
    // 换一份 YAML 就换一种解读：类别 alpha / beta 只存在于数据里，代码不认识它们
    let alpha = Artifact::from_yaml(
        r#"
name: alpha
document_type: alpha
criteria:
  intent: 测试类别甲
  rules: 文本命中「测试」时归 alpha。
title: {source: first_h1, fallback: filename}
description: {location: preface, meaning: 说明}
sections: {split_by: H2}
"#,
    )
    .expect("合法规则");
    let beta = Artifact::from_yaml(
        r#"
name: beta
document_type: beta
criteria:
  intent: 测试类别乙
  rules: 其余归 beta。
title: {source: first_h1, fallback: filename}
description: {location: preface, meaning: 说明}
sections: {split_by: H2}
"#,
    )
    .expect("合法规则");
    let engine = semantic_engine();

    // 语义客户端回在册的第一个类别——名字从判据里读，不从代码里读
    let entry = classify(&engine, "永远分类这句话", &[alpha, beta]);
    assert_eq!(entry.destination.as_deref(), Some("alpha"));
}

#[test]
fn classify_failure_is_conservative_and_visible() {
    // 判断失败（LLM 不可用）→ 留在 journal，理由写失败原因
    let engine = Engine::new(demo_llm());
    let entry = classify(&engine, "原来问题的关键是缓存", &categories());
    assert_eq!(entry.destination, None);
    assert_eq!(entry.destination_name(), JOURNAL);
    assert!(entry.reason.contains("判断失败"));
}

#[test]
fn classifies_in_batch() {
    let engine = semantic_engine();
    let texts = ["今天开了三个会".to_string(), "决定先做登录页".to_string()];
    let classified: Vec<_> = texts
        .iter()
        .map(|text| classify(&engine, text, &categories()))
        .collect();
    assert_eq!(classified.len(), 2);
    assert_eq!(classified[0].text, texts[0]);
    assert_eq!(classified[0].destination, None);
    assert_eq!(classified[1].destination.as_deref(), Some("intention"));
}

#[test]
fn grades_by_evidence() {
    let engine = semantic_engine();
    // LLM 首选：证据是输入，级别是对证据的判断——计数不是结论
    assert_eq!(
        grade(&engine, "x", 1, false, false),
        InsightGrade::new("confirmed")
    );
    assert_eq!(
        grade(&engine, "x", 5, true, true),
        InsightGrade::new("confirmed")
    );

    // 规则降级：LLM 不可用时按计数判
    let fallback = Engine::new(demo_llm());
    assert_eq!(
        grade(&fallback, "x", 1, false, false),
        InsightGrade::new("hypothesis")
    );
    assert_eq!(
        grade(&fallback, "x", 2, false, false),
        InsightGrade::new("confirmed")
    );
    assert_eq!(
        grade(&fallback, "x", 1, true, false),
        InsightGrade::new("confirmed")
    );
    assert_eq!(
        grade(&fallback, "x", 5, true, true),
        InsightGrade::new("hypothesis")
    );

    // 降级路径也可以直接调用
    assert_eq!(
        grade_by_rules(2, false, false),
        InsightGrade::new("confirmed")
    );
}

#[test]
fn clusters_by_essence_not_wording() {
    let a = InsightItem::new("缓存失效的规律是 TTL 太长", "读多写少适用");
    let b = InsightItem::new("缓存失效的规律其实是 TTL 太长", "写多读少不适用");
    let other = InsightItem::new("会议纪要要当天写完", "当天发群里");

    let engine = semantic_engine();
    let clustered = cluster(&engine, &[a.clone(), b, other]);
    assert_eq!(clustered.len(), 2);
    assert_eq!(clustered[0].statement, a.statement);
    assert_eq!(clustered[0].detail, "读多写少适用；写多读少不适用");

    // 单条不动
    assert_eq!(cluster(&engine, std::slice::from_ref(&a)).len(), 1);

    // 判断失败 → 保守不合并（不写坏数据）
    let fallback = Engine::new(demo_llm());
    let clustered = cluster(
        &fallback,
        &[
            a.clone(),
            InsightItem::new("缓存失效的规律其实是 TTL 太长", ""),
        ],
    );
    assert_eq!(clustered.len(), 2);
}

#[test]
fn merge_follows_four_strategies() {
    let existing = InsightItem::new("缓存失效的规律是 TTL 太长", "读多写少适用");
    let engine = semantic_engine();

    // 没有已有条目 → 机械新增，不用判断
    let decision = decide_merge(&engine, None, InsightItem::new("新命题", ""));
    assert_eq!(decision.action.as_str(), "add");
    assert_eq!(decision.target.statement, "新命题");
    assert!(decision.source.is_none());

    // 同本质 → 就地改写
    let twin = InsightItem::new("缓存失效的规律是 TTL 太长", "重复记了一次");
    let decision = decide_merge(&engine, Some(&existing), twin.clone());
    assert_eq!(decision.action.as_str(), "rewrite");
    assert_eq!(decision.target.detail, "读多写少适用；重复记了一次");

    // 被推翻 → 直接替换
    let refuted = InsightItem::new("计划不是先做 A", "");
    let plan = InsightItem::new("计划先做 A", "");
    let decision = decide_merge(&engine, Some(&plan), refuted);
    assert_eq!(decision.action.as_str(), "replace");
    assert_eq!(decision.target.statement, "计划不是先做 A");

    // 重复但不同本质 → 留互链，不重复收录
    let decision = decide_merge(
        &engine,
        Some(&existing),
        InsightItem::new("完全另一个命题", ""),
    );
    assert_eq!(decision.action.as_str(), "crossLink");
    assert_eq!(decision.target.statement, existing.statement);

    // 批量合并：改写不扩容、新增才扩容
    let merged = merge(
        &engine,
        vec![existing.clone()],
        vec![
            InsightItem::new("缓存失效的规律是 TTL 太长", "又记一次"),
            InsightItem::new("完全另一个命题", ""),
        ],
    );
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].detail, "读多写少适用；又记一次");
    assert_eq!(merged[1].statement, "完全另一个命题");

    // 判断失败 → 兜底互链（actions 末项），不破坏已有数据
    let fallback = Engine::new(demo_llm());
    let decision = decide_merge(&fallback, Some(&existing), twin);
    assert_eq!(decision.action.as_str(), "crossLink");
}

#[test]
fn promotion_and_fork_rules() {
    let engine = semantic_engine();
    let item = InsightItem::new("命题", "没有行动词");

    // 「稳定为思维框架」交给判断，套用次数只是证据
    assert!(should_promote_to_profile(&engine, &item, 2));
    // 命题引出选择 → 分流
    assert!(should_fork_to_roadmap(&engine, &item));

    // 规则降级：判断失败时按计数晋升、不分流
    let fallback = Engine::new(demo_llm());
    assert!(!should_promote_to_profile(&fallback, &item, 2));
    assert!(should_promote_to_profile(&fallback, &item, 3));
    assert!(!should_fork_to_roadmap(&fallback, &item));
}
