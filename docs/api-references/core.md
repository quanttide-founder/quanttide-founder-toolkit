# core 模块

跨域共享的部分：`parse` 不认业务，`rules` 只定义不执行，`engine` 只加工不做结构发现。域依赖 core，core 不依赖域。

## parse：解析与结构切分

文本 → 块序列 → Section 树。通用，任何 Markdown 文档都一样。

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~dart
// lib/src/core/parse.dart
enum BlockType { heading, bullet, paragraph, separator }

class Block {
  final BlockType type;
  final int level;   // 仅 heading 有效：1-6
  final String text;
}

class MarkdownDocument {
  final String path;
  final List<Block> blocks;
  static MarkdownDocument parse(String path, String content);
  String? get title; // 首个一级标题
}

class RawSection {
  final int level;
  final String title; // 首个元素是前言，title 为空串
  final List<Block> blocks;
  final List<RawSection> subsections;
}

List<RawSection> splitSections(List<Block> blocks, int level);

class TextSection {
  String title;
  final List<String> paragraphs, bullets;
  final List<TextSection> subsections;
  static TextSection fromRaw(RawSection raw);
}

class NamedItem {
  final String name, detail;
}

NamedItem parseNamedItem(String text); // `- **名称**：详情`，无冒号整句作 name
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~rust
// src/core/parse.rs
pub enum BlockType { Heading, Bullet, Paragraph, Separator }

pub struct Block {
    pub kind: BlockType,
    pub level: usize,  // 仅 heading 有效：1-6
    pub text: String,
}

impl MarkdownDocument {
    pub fn parse(path: impl Into<String>, content: &str) -> Self;
    pub fn title(&self) -> Option<&str>; // 首个一级标题
}

pub struct RawSection {
    pub level: usize,
    pub title: String, // 首个元素是前言，title 为空串
    pub blocks: Vec<Block>,
    pub subsections: Vec<RawSection>,
}

pub fn split_sections(blocks: &[Block], level: usize) -> Vec<RawSection>;

impl TextSection {
    pub fn from_raw(raw: &RawSection) -> Self;
}

pub fn parse_named_item(text: &str) -> NamedItem;
pub struct NamedItem { pub name: String, pub detail: String }
~~~

:::
```

## rules：规则与工作流加载

规则文件描述一种文档怎么读，工作流描述一个任务怎么做；YAML 缺省字段按默认值处理，格式错误两侧都报错。

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~dart
// lib/src/core/rules.dart
class Artifact {
  final String documentType;
  final Map<String, dynamic> title, description, sections;
  static Artifact fromFile(String path);
  String get instruction; // 规则的自然语言描述，供 LLM 理解
}

class Workflow {
  final String name, description, input, rules;
  final List<String> output;
  final List<Step> steps;
  static Workflow fromFile(String path);
}

class Step {
  final String verb;        // 只能是 scan / judge / merge
  final String description;
}
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~rust
// src/core/rules.rs
pub struct Artifact {
    pub document_type: String,
    pub title: TitleRule,               // source / fallback
    pub description: DescriptionRule,   // location / meaning
    pub sections: SectionsRule,         // split_by / subsections / extract / unknown_content
}
impl Artifact {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Error>;
    pub fn from_yaml(content: &str) -> Result<Self, Error>;
    pub fn instruction(&self) -> String;
}

pub struct Workflow {
    pub name: String,
    pub description: String,
    pub input: String,
    pub output: Vec<String>,
    pub steps: Vec<Step>,
    pub rules: String,
}
impl Workflow {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Error>;
    pub fn from_yaml(content: &str) -> Result<Self, Error>;
}

pub struct Step { pub verb: String, pub description: String }
// YAML 里写成单键映射 `- scan: 描述`，自定义 Deserialize 取首个键值对
~~~

:::
```

## engine：提取器与三个动词

提取方式可插拔：规则提取是无 LLM 时的降级方案，LLM 提取是首选。三个动词中 `judge` 走 LLM，其余是代码。

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~dart
// lib/src/core/engine.dart
class ExtractionResult {
  final String? title, description;
  final List<Map<String, dynamic>> sections;
  Map<String, dynamic> toJson();
}

abstract class SemanticExtractor {
  ExtractionResult extract({required List<RawSection> sections, required Artifact rule});
}

class RuleBasedExtractor implements SemanticExtractor;      // 纯规则匹配
class LlmExtractor implements SemanticExtractor {           // LLM 按规则填表
  LlmExtractor({required LlmClient client});
}

// lib/src/core/llm.dart —— 由使用方实现
abstract class LlmClient {
  String complete(String prompt);
}

class Engine {
  Engine({required LlmClient llmClient});
  WorkflowResult run(Workflow workflow, List<dynamic> input);
  List<Candidate> scan(List<dynamic> items, List<String> keywords);
  Judgment judge(String rules, List<dynamic> items);
  List<dynamic> merge(List<dynamic> existing, List<dynamic> incoming);
}

class WorkflowResult { final List<dynamic> items; final List<Judgment> judgments; }
class Judgment { final String decision; final String? reason; }
class Candidate { final String source, text; }
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~rust
// src/core/engine.rs
pub struct ExtractionResult {
    pub title: Option<String>,
    pub description: Option<String>,
    pub sections: Vec<serde_json::Map<String, Value>>,
}
impl ExtractionResult { pub fn to_json(&self) -> Value; }

pub trait SemanticExtractor {
    fn extract(&self, sections: &[RawSection], rule: &Artifact)
        -> Result<ExtractionResult, Error>;
}

pub struct RuleBasedExtractor;                     // 纯规则匹配
pub struct LlmExtractor { pub llm: LLM }           // LLM 按规则填表
impl LlmExtractor { pub fn new(llm: LLM) -> Self; }

// LLM 客户端来自 quanttide-agent：LLM::default() 读 LLM_* 环境变量，
// 测试与演示用 LLM::with_client 注入桩客户端

pub struct Engine { pub llm: LLM }
impl Engine {
    pub fn new(llm: LLM) -> Self;
    pub fn run(&self, workflow: &Workflow, input: Vec<Value>)
        -> Result<WorkflowResult, Error>;          // 未知动词报 Error::UnknownVerb
    pub fn scan(items: &[Value], keywords: &[String]) -> Vec<Candidate>;
    pub fn judge(&self, rules: &str, items: &[Value]) -> Result<Judgment, Error>;
    pub fn merge(existing: &[Value], incoming: &[Value]) -> Vec<Value>;
}

pub struct WorkflowResult { pub items: Vec<Value>, pub judgments: Vec<Judgment> }
pub struct Judgment { pub decision: String, pub reason: Option<String> }
pub struct Candidate { pub source: String, pub text: String }
~~~

:::
```

`scan` 的候选来源是条目序号，`judge` 的返回去掉首尾空白；`run` 里 `scan` 步的关键词表是占位空表（与 Dart 侧一致），由调用方指定或从 `rules` 解析后再接。
