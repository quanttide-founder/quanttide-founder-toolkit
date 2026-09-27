# fiction 模块

观察站素材 → 阶段 → 成稿：`models` 是纯数据与目录发现，`repository` 只装载，`states` 是三步提炼与结构判断。

## models：小说与观察站

阶段目录 `{N}_{名称}` 动态识别，不硬编码阶段名；章节文件 `{序号}_{标题}.md`，`0_` 前缀是前言不占正文章节号，未编号是替代草稿。

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~dart
// lib/src/fiction/models.dart
class Stage {
  final int number;            // 目录名前缀
  final String name;           // 目录名后缀
  final Directory directory;
  final List<Chapter> chapters;
  static List<Stage> discover(Directory novelRoot);
}

class Chapter {
  final int? number;           // null 表示未编号（替代草稿）
  final String title;
  final File file;
  final String content;
  bool get isPreface;          // 0_ 前言等非正文
  bool get isUnnumbered;
  static List<Chapter> discover(Directory stageDir); // 跳过 README
}

class Novel {
  final String name;
  final Directory directory;
  final List<Stage> stages;
  final String? indexContent;  // index.md 原文
  Iterable<Chapter> get allChapters;      // 跨阶段共用编号轴
  Stage? stageByName(String name);
}

class EmotionalDiary { final String title; final File file; final String content; }
class SocialObservation { final String title; final File file; final String content; }
class Observation {
  final List<EmotionalDiary> emotionalDiaries;
  final List<SocialObservation> socialObservations;
}
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~rust
// src/fiction/models.rs
pub struct Stage {
    pub number: u32,           // 目录名前缀
    pub name: String,          // 目录名后缀
    pub directory: PathBuf,
    pub chapters: Vec<Chapter>,
}
impl Stage {
    pub fn is_stage_dir_name(name: &str) -> bool;
    pub fn discover(novel_root: impl AsRef<Path>) -> Result<Vec<Stage>, Error>;
}

pub struct Chapter {
    pub number: Option<u32>,   // None 表示未编号（替代草稿）
    pub title: String,
    pub file: PathBuf,
    pub content: String,
}
impl Chapter {
    pub fn is_preface(&self) -> bool;    // 0_ 前言等非正文
    pub fn is_unnumbered(&self) -> bool;
    pub fn discover(stage_dir: impl AsRef<Path>) -> Result<Vec<Chapter>, Error>; // 跳过 README
}

pub struct Novel {
    pub name: String,
    pub directory: PathBuf,
    pub stages: Vec<Stage>,
    pub index_content: Option<String>,   // index.md 原文
}
impl Novel {
    pub fn all_chapters(&self) -> Vec<&Chapter>;      // 跨阶段共用编号轴
    pub fn stage_by_name(&self, name: &str) -> Option<&Stage>;
}

pub struct EmotionalDiary { pub title: String, pub file: PathBuf, pub content: String }
pub struct SocialObservation { pub title: String, pub file: PathBuf, pub content: String }
pub struct Observation {
    pub emotional_diaries: Vec<EmotionalDiary>,
    pub social_observations: Vec<SocialObservation>,
}
~~~

:::
```

## repository：装载

小说目录 = 含 `index.md` 或 `N_` 阶段子目录的目录；`观察站/` 单独解析成情绪日记（`1_情绪日记/`）与社会观察（`2_社会观察/`），`实验室/` 跳过。

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~dart
// lib/src/fiction/repository.dart
class FictionRepository {
  final Directory root;
  final List<Novel> novels;
  final Observation observation; // 无观察站时为空列表
  static FictionRepository load(String rootPath);
}
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~rust
// src/fiction/repository.rs
pub struct FictionRepository {
    pub root: PathBuf,
    pub novels: Vec<Novel>,
    pub observation: Observation, // 无观察站时为空列表
}
impl FictionRepository {
    pub fn load(root: impl AsRef<Path>) -> Result<Self, Error>;
}
~~~

:::
```

## states：三步提炼与结构判断

三步提炼走状态机（取样 → 观察展开 → 落实片段，中间那步不能省）；编号轴、阶段流转、包装文案是单发判断，两侧都是函数调用。

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~dart
// lib/src/fiction/bloc.dart
class ExtractedFragment {
  final String motif;   // 母题卡片：一句话说清写什么
  final String scene;   // 场景素材：50 字钩子
  final String source;  // 来源素材标题
}

class NumberAssignment {
  final int assigned;
  final int? gap;       // null = 追加到最大号 +1
}

class StageStatus {
  final bool isComplete;
  final List<int> missingNumbers;
  final int totalChapters;
}

class FictionBloc {
  FictionBloc({required Engine engine});
  ExtractedFragment extract(EmotionalDiary diary);
  List<ExtractedFragment> extractAll(List<EmotionalDiary> diaries);
  NumberAssignment assignNumber(Novel novel);
  List<int> findGaps(Novel novel);
  StageStatus checkStage(Stage stage);
  List<String> stageFlow(Novel novel);
  Map<String, String> extractPackaging(String content); // title / tagline / theme
}
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~rust
// src/fiction/states.rs
pub struct ExtractedFragment {
    pub motif: String,   // 母题卡片：一句话说清写什么
    pub scene: String,   // 场景素材：50 字钩子
    pub source: String,  // 来源素材标题
}
pub struct NumberAssignment { pub assigned: u32, pub gap: Option<u32> }
pub struct StageStatus { pub is_complete: bool, pub missing_numbers: Vec<u32>,
                         pub total_chapters: usize }
pub struct Packaging { pub title: String, pub tagline: String, pub theme: String }

// 状态机：驱动它需要 use quanttide_founder::statig::prelude::*
pub struct FictionFlow;
impl FictionFlow {
    pub fn extract(diary: &EmotionalDiary) -> ExtractedFragment;
    pub fn extract_all(diaries: &[EmotionalDiary]) -> Vec<ExtractedFragment>;
}
pub enum Event { Sample(EmotionalDiary), Expand, Settle }
// 根重导出为 FictionEvent；FictionState：Idle / Sampled / Observed / Fragmented

// 结构判断：自由函数
pub fn assign_number(novel: &Novel) -> NumberAssignment;
pub fn find_gaps(novel: &Novel) -> Vec<u32>;
pub fn check_stage(stage: &Stage) -> StageStatus;
pub fn stage_flow(novel: &Novel) -> Vec<String>;
pub fn extract_packaging(content: &str) -> Packaging;
~~~

:::
```

包装文案的返回有形态差异：Dart 是 `Map<String, String>`（键 `title` / `tagline` / `theme`），Rust 是 `Packaging` 结构体。
