# ROADMAP

Rust 包（quanttide_founder）路线图。问题分析见 [docs/index.md](docs/index.md)，结论：设计完整、执行降级——九处「需要理解」被填成了字符串匹配，根源是三分法（type / category / tag）没有落地。

## 原则

- 重构从意图的可判定化开始：先写判据，再改类型；类型是判据的产物。
- 每步只动一个东西，不重写整个项目。
- 需要语义判断的地方接 core 已有的接口（`Engine::judge` / `LlmExtractor`），规则版只作降级。
- 从最明显的坏点 `route` 开始——它牵动最多，修它会连锁暴露整串结构问题。

## 阶段一：写判据（不改代码）

- [ ] `Destination` 五个类别各写判据：意图、判据、边界（正例 / 反例 / 易混例），易混例必须给出裁决
- [ ] `InsightGrade` 分级判据（已确认 / 假说 / 被推翻）
- [ ] 「本质相同」判据（供 `cluster` / `is_same_essence` 用）
- [ ] 「情绪 → 外部观察」判据（供 `expand_observation`，不到 `Observed` 不给 `Settle`）
- [ ] 「稳定为思维框架」判据（供 `should_promote_to_profile`）
- [ ] 母题、包装文案、具体细节、被推翻各自的判据

## 阶段二：Destination 改成 Artifact 的 key

- [ ] 拆掉 `enum Destination`，四个变体变成四份 `Artifact.name`（insight / profile / roadmap / intention）
- [ ] `Journal` 改为补集：`route(text, artifacts) -> Option<&Artifact>`，`None` 即留在 journal
- [ ] 判据写进 Artifact YAML，`rules.rs` 真正被加载、被使用（当前悬空）
- [ ] `route` 签名改为 `fn(text, artifacts) -> Option<&Artifact>`，`text.contains("要")` 失去落脚点

## 阶段三：分类收成一次判断

- [ ] 合并 `route` + `classify` 为单条 `classify(text)`，批量由调用方 map，不留两层概念
- [ ] type + category 一次语义判断同时输出；tag 单独走元数据提取，不捆进语义判断
- [ ] 判据交给 `Engine::judge` 执行，删除五张关键词表（`COGNITION` / `INTENT` / `DIRECTION` / `PROFILE` 等）

## 阶段四：其余降级逐个接回 core 接口

| 位置 | 意图 | 目标接口 |
|:--|:--|:--|
| `grade` | 证据分级 | `Engine::judge(grade_rules, ...)` |
| `cluster` / `is_same_essence` | 本质相同合并 | `Engine::judge(cluster_rules, items)` |
| `expand_observation` | 情绪提取外部观察 | `LlmExtractor::extract` |
| `to_fragment` | 提炼母题 | `LlmExtractor::extract` |
| `extract_packaging` | 提取包装文案 | `LlmExtractor::extract` |
| `has_concrete_detail` | 含具体细节判断 | 判据 + `Engine::judge` |
| `is_refuted` | 判断被推翻 | `Engine::judge` |

- [ ] 每替换一处，同步把对应判据落进 YAML，规则版 `RuleBasedExtractor` 降为 fallback 而非主实现
- [ ] 主次关系显式化，不让 trait 把「LLM 首选、规则降级」抹成平级

## 阶段五：同类错位清理（三分类落地）

- [ ] category → type：`InsightGrade`、`MergeAction`、`Step.verb` 改为数据驱动
- [ ] type → String：`split_by`、`title.source`、`description.location`、`Step.verb` 改回固定取值，加载期校验（`split_by: "h7"` 应当加载失败）
- [ ] category → 计数：`should_promote_to_profile`、`grade` 的 `occurrence_count >= 2` 改为语义判断，计数只作证据输入
- [ ] category → 关键词表：13 个单字动词等硬编码判据随阶段三、四删除
- [ ] 缺失语义：`reason: None` 这类 `Option<String>` 改为必填或显式「无」，不让缺失看起来正常

## 阶段六：执行缺陷修复

- [ ] `Engine::run` 的 `scan` 清空 `items` 的问题
- [ ] `RuleBasedExtractor` 的 memory 约定（前言段 → description）移出 core 或改为配置

## 验证

- [ ] `cargo test` / `cargo clippy` 通过，`examples/` 五个示例可运行
- [ ] 判据进 YAML 后，换一份规则即换一种解读方式，无需改代码
- [ ] 与 Dart 包行为对齐（LLM 调用方式、域编排方式两处按 Rust 另选实现）
