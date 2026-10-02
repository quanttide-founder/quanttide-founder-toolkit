# quanttide_founder — Dart 包

量潮创始人工具箱的 Dart 实现。通用逻辑（人机交互框架定位、读写与计算、规则设计）见 [toolkit 文档](../../../docs/index.md)。

## 目录结构

```
lib/src/
├── core/            跨域共享（域依赖 core，core 不依赖域）
│   ├── parse.dart       Markdown 解析 + 结构切分
│   ├── rules.dart       Artifact / Workflow 加载
│   ├── engine.dart      scan / judge / merge
│   └── llm.dart         LlmClient
├── memory/          memory 域
│   ├── models.dart      JournalEntry / InsightDoc / ProfileDoc / RoadmapDoc
│   ├── repository.dart  MemoryRepository
│   └── bloc.dart        工作流编排
└── fiction/         fiction 域
    ├── models.dart      Novel / Chapter / Stage / Observation
    ├── repository.dart  FictionRepository
    └── bloc.dart        工作流编排

assets/
├── artifacts/       名词定义（YAML）
└── workflows/       动词定义（YAML）
```

按 Bloc 架构组织：feature-first（域目录是主体）+ 域内数据与逻辑分离（models / repository / bloc 三个文件）。跨域共享收在 `core/`，域依赖 core，core 不依赖域。

## 分层职责

| 层 | 回答 | 不做什么 |
|---|---|---|
| `core/parse.dart` | 文本长什么样 | 不认业务 |
| `core/rules.dart` | 名词和任务怎么定义 | 不执行 |
| `core/engine.dart` | 怎么加工（scan / judge / merge） | 不管结构发现 |
| `{域}/models.dart` | 有什么 | 不读文件 |
| `{域}/repository.dart` | 从哪装载 | 不做语义 |
| `{域}/bloc.dart` | 做成什么 | 不管 I/O |

## 资产定义

**artifact**（名词）定义一个语义模型长什么样、从哪提取：

```yaml
# tests/fixtures/artifacts/journal.yaml
name: journal
description: 时间线——原始记录
source:
  directories: ["<集>/", "<集>/journal/"]
  file_pattern: "YYYY-MM-DD.md"
fields:
  - name: date
    from: 文件名
  - name: content
    from: 正文
```

**workflow**（动词）定义一个任务怎么做，用引擎的三个动词组装：

```yaml
# tests/fixtures/workflows/classify.yaml
name: classify
description: 三问过滤——判断日志条目的去向
input: journal
output: [insight, profile, roadmap, journal]
steps:
  - scan: 按认知关键词定位候选
  - judge: 三问判断去向，拿不准不收
rules: |
  「想通了什么」→ insight、「我是谁」→ profile、
  「要做什么」→ roadmap、「发生了什么」→ 留 journal。
```

**引擎契约**：workflow 的 `steps` 只能用三个动词——`scan`（关键词定位，代码）、`judge`（LLM 判断）、`merge`（策略合并，代码）。加新动词改引擎；加新任务加 workflow YAML；加新名词加 artifact YAML。

## 开发

```sh
dart pub get
dart analyze
dart test
```
