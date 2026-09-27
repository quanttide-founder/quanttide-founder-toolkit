# memory 模块

时间线 → 特征 → 认知 → 方向四层：`models` 是纯数据，`repository` 只装载，`states` 是四步流程与判断规则。

## models：四层域模型

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~dart
// lib/src/memory/models.dart
enum JournalSource { root, archive }

class JournalEntry {
  final DateTime date;          // 文件名即日期
  final String path;
  final JournalSource source;   // 集根当天 / journal 归档
  final String content;
  List<String> get segments;    // 按 --- 分隔线切出的会话段
  int get characterCount;       // 去掉全部空白后的字数
}

class ProfileDoc {
  final String path, name, description;
  final String? title;          // 一级标题，缺失时由文件名兜底
  final List<TextSection> sections;
  static ProfileDoc parse(File file);
}

enum InsightGrade { confirmed, hypothesis }

class InsightItem {
  final String statement;       // 命题
  final String detail;          // 解释
  final String? evidence;       // 「依据：」之后的证据
}

class InsightSection {
  String title;
  final InsightGrade? grade;    // 主题式节为 null
  final List<InsightItem> items;
  final List<String> paragraphs;
}

class InsightDoc {
  final String path, name;
  final String? title;
  final List<InsightSection> sections;
  Iterable<InsightItem> itemsOf(InsightGrade grade);
  static InsightDoc parse(File file);
}

class RoadmapDoc {
  final String path, name;
  final String? title, goal;
  final List<NamedItem> metaGoals, coreProblems, decided, pending;
  final List<TextSection> themeSections; // 认不出的节原样保留
  static RoadmapDoc parse(File file);
}
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~rust
// src/memory/models.rs
pub enum JournalSource { Root, Archive }

pub struct JournalEntry {
    pub date: NaiveDate,        // 文件名即日期
    pub path: String,
    pub source: JournalSource,  // 集根当天 / journal 归档
    pub content: String,
}
impl JournalEntry {
    pub fn segments(&self) -> Vec<&str>;      // 按 --- 分隔线切出的会话段
    pub fn character_count(&self) -> usize;    // 去掉全部空白后的字数
}

pub struct ProfileDoc {
    pub path: String,
    pub name: String,
    pub title: Option<String>,   // 一级标题，缺失时由文件名兜底
    pub description: String,
    pub sections: Vec<TextSection>,
}
impl ProfileDoc {
    pub fn parse(path: impl Into<String>, content: &str) -> Self; // 不读文件，内容由调用方给
}

pub enum InsightGrade { Confirmed, Hypothesis }

pub struct InsightItem {
    pub statement: String,       // 命题
    pub detail: String,          // 解释
    pub evidence: Option<String>,// 「依据：」之后的证据
}

pub struct InsightSection {
    pub title: String,
    pub grade: Option<InsightGrade>, // 主题式节为 None
    pub items: Vec<InsightItem>,
    pub paragraphs: Vec<String>,
}

impl InsightDoc {
    pub fn parse(path: impl Into<String>, content: &str) -> Self;
    pub fn items_of(&self, grade: InsightGrade) -> impl Iterator<Item = &InsightItem>;
}

impl RoadmapDoc {
    pub fn parse(path: impl Into<String>, content: &str) -> Self;
    // 字段同 Dart：goal / meta_goals / core_problems / decided / pending / theme_sections
}
~~~

:::
```

## repository：装载

根目录下含任一特征子目录（`journal`、`profile`、`insight`、`roadmap`）的目录算一个记忆集；日志只认 `YYYY-MM-DD.md`，层目录跳过 `README.md`。

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~dart
// lib/src/memory/repository.dart
class MemorySet {
  final String name;
  final Directory root;
  final List<JournalEntry> journals; // 按日期倒序
  final List<ProfileDoc> profiles;
  final List<InsightDoc> insights;
  final List<RoadmapDoc> roadmaps;
}

class MemoryRepository {
  final Directory root;
  final List<MemorySet> sets;
  static MemoryRepository load(String rootPath); // 根不存在抛 FileSystemException
  List<JournalEntry> get allJournals;            // 全部日志，按日期倒序
}
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~rust
// src/memory/repository.rs
pub struct MemorySet {
    pub name: String,
    pub root: PathBuf,
    pub journals: Vec<JournalEntry>,   // 按日期倒序
    pub profiles: Vec<ProfileDoc>,
    pub insights: Vec<InsightDoc>,
    pub roadmaps: Vec<RoadmapDoc>,
}

pub struct MemoryRepository {
    pub root: PathBuf,
    pub sets: Vec<MemorySet>,
}
impl MemoryRepository {
    pub fn load(root: impl AsRef<Path>) -> Result<Self, Error>; // 根不存在报 Error::RootNotFound
    pub fn all_journals(&self) -> Vec<&JournalEntry>;           // 全部日志，按日期倒序
}
~~~

:::
```

## states：四步流程与判断规则

编排逻辑（三问优先级、证据分级、合并四策略、升降级）两侧同一套；差别只在载体：Dart 是 `MemoryBloc` 实例方法，Rust 是状态机加自由函数。

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~dart
// lib/src/memory/bloc.dart
enum Destination { insight, profile, roadmap, intention, journal }
enum MergeAction { add, rewrite, replace, crossLink }

class ClassifiedEntry {
  final String text;
  final Destination destination;
  final String? reason;
}
class GradedInsight { final InsightItem item; final InsightGrade grade; }
class MergeDecision {
  final MergeAction action;
  final InsightItem target;
  final InsightItem? source;
}

class MemoryBloc {
  MemoryBloc({required Engine engine});
  List<ClassifiedEntry> processJournal(Workflow workflow, List<JournalEntry> entries);
  Destination route(String text);
  List<ClassifiedEntry> classify(List<String> texts);
  InsightGrade grade(String text,
      {int occurrenceCount = 1, bool hasVerification = false, bool isRefuted = false});
  List<InsightItem> cluster(List<InsightItem> items);
  MergeDecision decideMerge(InsightItem? existing, InsightItem incoming);
  List<InsightItem> merge(List<InsightItem> existing, List<InsightItem> incoming);
  bool shouldPromoteToProfile(InsightItem item, {int applicationCount = 0});
  bool shouldForkToRoadmap(InsightItem item);
}
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~rust
// src/memory/states.rs
pub enum Destination { Insight, Profile, Roadmap, Intention, Journal } // as_str()
pub enum MergeAction { Add, Rewrite, Replace, CrossLink }             // as_str()
pub struct ClassifiedEntry { pub text: String, pub destination: Destination,
                             pub reason: Option<String> }
pub struct GradedInsight { pub item: InsightItem, pub grade: InsightGrade }
pub struct MergeDecision { pub action: MergeAction, pub target: InsightItem,
                           pub source: Option<InsightItem> }

// 状态机：驱动它需要 use quanttide_founder::statig::prelude::*
pub struct MemoryFlow;
impl MemoryFlow {
    pub fn process_journal(entries: &[JournalEntry]) -> Vec<ClassifiedEntry>;
}
pub enum Event { Scan(Vec<JournalEntry>), Classify(Vec<String>), Route, Cluster }
// MemoryState：Idle / Located{candidates} / Routed{classified} / Clustered{classified}
impl MemoryState {
    pub fn candidates(&self) -> Option<&[String]>;
    pub fn classified(&self) -> Option<&[ClassifiedEntry]>;
}

// 判断规则：自由函数
pub fn route(text: &str) -> Destination;
pub fn classify(texts: &[String]) -> Vec<ClassifiedEntry>;
pub fn grade(_text: &str, occurrence_count: usize,
             has_verification: bool, is_refuted: bool) -> InsightGrade;
pub fn cluster(items: &[InsightItem]) -> Vec<InsightItem>;
pub fn decide_merge(existing: Option<&InsightItem>, incoming: InsightItem) -> MergeDecision;
pub fn merge(existing: Vec<InsightItem>, incoming: Vec<InsightItem>) -> Vec<InsightItem>;
pub fn should_promote_to_profile(_item: &InsightItem, application_count: usize) -> bool;
pub fn should_fork_to_roadmap(item: &InsightItem) -> bool;
~~~

:::
```

两处签名保留了未参与判定的文本参数（`grade` 的 `_text`、`should_promote_to_profile` 的 `_item`），与 Dart 侧同形；`process_journal` 不收 Dart 侧未参与计算的 `Workflow` 参数，流程定义即状态机本身。
