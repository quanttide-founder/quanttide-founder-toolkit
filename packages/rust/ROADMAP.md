# ROADMAP

Rust 包（quanttide_founder）路线图。问题分析见 [docs/index.md](docs/index.md)，判据见 [docs/criteria.md](docs/criteria.md)。结论：设计完整、执行降级——九处「需要理解」被填成了字符串匹配，根源是三分法（type / category / tag）没有落地。

状态：全部完成，验收通过（2026-10-02）。`[x]` = 已完成，验收记录见文末。

## 原则

- 重构从意图的可判定化开始：先写判据，再改类型；类型是判据的产物。
- 每步只动一个东西，不重写整个项目。
- 需要语义判断的地方接 core 已有的接口（`Engine::judge` / `LlmExtractor`），规则版只作降级。
- 从最明显的坏点 `route` 开始——它牵动最多，修它会连锁暴露整串结构问题。

## 阶段一：写判据（不改代码）

- [x] `Destination` 五个类别各写判据：意图、判据、边界（正例 / 反例 / 易混例），易混例给出裁决
- [x] `InsightGrade` 分级判据（已确认 / 假说 / 被推翻）
- [x] 「本质相同」判据（供 `cluster` / `is_same_essence` 用）
- [x] 「情绪 → 外部观察」判据（供 `expand_observation`，不到 `Observed` 不给 `Settle`）
- [x] 「稳定为思维框架」判据（供 `should_promote_to_profile`）
- [x] 母题、包装文案、具体细节、被推翻各自的判据

## 阶段二：Destination 改成 Artifact 的 key

- [x] 拆掉 `enum Destination`，四个变体变成四份 `Artifact.name`（insight / profile / roadmap / intention）
- [x] `Journal` 改为补集：`ClassifiedEntry.destination` 为 `Option`，`None` 即留在 journal（`JOURNAL` 常量供展示）
- [x] 判据写进 Artifact YAML，`rules.rs` 真正被加载、被使用（编译期嵌入 + 运行期 `from_file`）
- [x] 分类签名改为数据驱动的 `classify(engine, text, categories)`，`text.contains("要")` 失去落脚点

## 阶段三：分类收成一次判断

- [x] 合并 `route` + `classify` 为单条 `classify(text)`，批量由调用方 map（状态机内循环），不留两层概念
- [x] category 一次语义判断同时输出（去向 + 必填理由）；tag 单独走元数据提取（`JournalEntry::tags`：日期从文件名来，来源从路径来）；type 由解析期块类型回答
- [x] 判据交给 `Engine::judge` 执行，删除五张关键词表（`COGNITION` / `INTENT` / `DIRECTION` / `PROFILE` / `ALL`）

## 阶段四：其余降级逐个接回 core 接口

| 位置 | 意图 | 目标接口 | 状态 |
|:--|:--|:--|:--|
| `grade` | 证据分级 | `Engine::judge(grade_rules, ...)` | [x] 计数降为证据输入，`grade_by_rules` 是显式降级 |
| `cluster` / `is_same_essence` | 本质相同合并 | `Engine::judge(cluster_rules, items)` | [x] 失败保守不合并，分词重合度已删 |
| `expand_observation` | 情绪提取外部观察 | `LlmExtractor::extract`（经 `Engine::extract`） | [x] 规则降级原样带下样本 |
| `to_fragment` | 提炼母题 | `LlmExtractor::extract` | [x] 降级：母题取素材标题、场景截 50 字 |
| `extract_packaging` | 提取包装文案 | `LlmExtractor::extract` | [x] 降级：`packaging_by_rules` 句子位置规则 |
| `has_concrete_detail` | 含具体细节判断 | 判据 + `Engine::judge` | [x] 13 个单字动词已删，降级取首句 |
| `is_refuted` | 判断被推翻 | `Engine::judge` | [x] 「不是」子串已删，失败按未推翻处理 |

- [x] 每替换一处，判据同步落进 YAML；规则版 `RuleBasedExtractor` / 规则函数降为 fallback 而非主实现
- [x] 主次关系显式化（trait 与函数文档标明 LLM 首选、规则降级），不再被 trait 抹成平级

## 阶段五：同类错位清理（三分类落地）

- [x] category → type：`InsightGrade`（名字来自 `insight.yaml` 的 `grades`）、`MergeAction`（名字来自 `cluster.yaml` 的 `actions`）、`Step.verb`（`Verb` 枚举）
- [x] type → String：`split_by`（`SplitLevel`）、`title.source`（`TitleSource`）、`description.location`（`DescriptionLocation`）、`Step.verb` 加载期校验（`split_by: h7` 加载即失败）
- [x] category → 计数：`should_promote_to_profile` 语义判断、次数降为证据（失败按计数降级）；`grade` 同上
- [x] category → 关键词表：13 个单字动词与五张关键词表随阶段三、四删除
- [x] 缺失语义：`ClassifiedEntry.reason`、`Judgment.reason` 改为必填，判断失败写明失败原因

## 阶段六：执行缺陷修复

- [x] `Engine::run` 的 `scan` 清空 `items`：scan 改为定位候选进 `WorkflowResult.candidates`，条目不动；关键词从 workflow 的 `rules` 文本读出（关键词是数据）
- [x] `RuleBasedExtractor` 的「前言段 → description」改为配置（`description.location` 决定提取位置，`none` 则不提取）

## 验证

- [x] `cargo test` 56 个测试通过；`cargo clippy --all-targets` 无本包告警；`cargo fmt --check` 通过
- [x] 判据进 YAML 后，换一份规则即换一种解读方式，无需改代码（`other_rules_reparse_the_same_document`、`classify_follows_supplied_categories_not_code`、`loads_shared_assets_end_to_end`）
- [x] 与 Dart 包行为对齐：解析、四步/三步流程、编号轴逐条钉住；语义判断按「LLM 调用方式」差异实现——Dart 侧仍是关键词，Rust 侧经 `Engine::judge` 走 `tests/fixtures/` 判据（域编排差异仍是 statig 状态机）

## 验收记录

2026-10-02，`packages/rust`：

- 测试：`cargo test` 56 通过（engine 12 / fiction 9 / memory 20 / parse 7 / rules 8）；`cargo fmt --check`、`cargo clippy --all-targets` 干净（仅依赖 `proc-macro-error2` 的 future-incompat 提示）
- 示例：`parse_memory` / `parse_fiction` / `agent_subscribe` 直接通过；`memory_workflow`、`fiction_workflow` 对主仓库真实数据集跑通（配置了 `LLM_API_KEY` 走真实判断，分类理由逐条可见；未配置则走显式降级）
- 资产：共享 YAML 迁至 toolkit 根 `tests/fixtures/`（原 `packages/dart/assets/`），Rust 编译期嵌入、测试按 `from_file` 端到端读取
