# quanttide_founder（Rust）

量潮创始人工具箱（quanttide-founder-toolkit）的 Rust 包，提供面向创始人角色的事务与工具能力。

与 Dart 包 `quanttide_founder` 行为对齐（memory 解析、fiction 解析、语义提取、四步/三步编排），两处按 Rust 的做法另选实现：

- **LLM 调用**不自带客户端接口，直接用 [quanttide-agent](https://crates.io/crates/quanttide-agent) 的 `LLM`——`Engine::judge` 与 `LlmExtractor` 按判据向它要判断与填表结果；判据是 YAML 数据（`tests/fixtures/`），语义处不再有关键词匹配
- **域编排**不用手写流转，用 [statig](https://crates.io/crates/statig) 状态机——事件进、状态出，走到哪一步一目了然；判断器作为上下文随事件进场

通用逻辑（人机交互框架定位、读写与计算、规则设计）见 [toolkit 文档](../../docs/index.md)。

## 安装

```toml
[dependencies]
quanttide-founder = "0.1.0-alpha.2"
```

## 模块

```
src/
├── core/            跨域共享（域依赖 core，core 不依赖域）
│   ├── parse.rs         Markdown 解析 + 结构切分
│   ├── rules.rs         Artifact / Workflow 加载（判据、分级、固定取值均加载期校验）
│   └── engine.rs        语义提取 + scan / judge / merge
├── memory/          memory 域
│   ├── models.rs        JournalEntry / InsightDoc / ProfileDoc / RoadmapDoc / Tag
│   ├── repository.rs    MemoryRepository
│   ├── rules.rs         内置规则资产（类别、判据、分级都是数据）
│   └── states.rs        四步流程状态机 + 判断规则（分类/分级/聚类/合并/升降级）
├── fiction/         fiction 域
│   ├── models.rs        Novel / Chapter / Stage / Observation
│   ├── repository.rs    FictionRepository
│   ├── rules.rs         内置规则资产（取样/观察/片段/包装的判据与字段）
│   └── states.rs        三步提炼状态机 + 结构判断（编号轴/阶段流转/包装文案）
└── error.rs         统一错误
```

## 状态机

memory 四步流程与 fiction 三步提炼都是事件驱动的状态机，中间产物放在状态里：

```text
memory：  Idle ──Scan──▶ Located ──Route──▶ Routed ──Cluster──▶ Clustered
fiction： Idle ──Sample─▶ Sampled ──Expand─▶ Observed ──Settle─▶ Fragmented
```

来早了的事件不理会（没扫描就路由、没取样就落实，状态原地不动）。

```rust
use quanttide_founder::{MemoryEvent, MemoryFlow};
use quanttide_founder::core::engine::Engine;
use quanttide_founder::statig::prelude::*;

let mut engine = Engine::new(llm); // 判断器作为上下文进场
let mut machine = MemoryFlow.state_machine();
machine.handle_with_context(&MemoryEvent::Classify(texts), &mut engine);
if let Some(classified) = machine.state().classified() {
    println!("{} 条", classified.len());
}
```

判断规则（`classify` / `grade` / `decide_merge` / `cluster`…）吃 `&Engine`：
LLM 按 YAML 判据首选，失败走显式规则降级；结构判断（`assign_number` / `find_gaps`…）是纯函数。

### 订阅状态流

工具箱的领域流水线是「取」：驱动完读一次 `machine.state()`（parse / workflow 四个示例）。
**进程内有多个消费者要实时知情时才是「推」**——状态每次变化推给订阅者
（对照 BLoC 的 `bloc.stream.listen`）。这属应用形态，不进工具箱，
故由示例自带机器演示：[`examples/agent_subscribe.rs`](examples/agent_subscribe.rs)
（Agent 驱动任务状态机，进度显示 / 流水记账 / 闸门提示各自订阅、各自退订，
驱动方对观察方一无所知——statig 的 `after_transition` 钩子推快照）。

## LLM

`Engine` 与 `LlmExtractor` 用 quanttide-agent 的 `LLM`，配置读它的环境变量：

```rust
use quanttide_founder::{Artifact, Engine, LlmExtractor, SemanticExtractor};
use quanttide_agent::LLM;

let extractor = LlmExtractor::new(LLM::default()); // LLM_MODEL / LLM_BASE_URL / LLM_API_KEY
let result = extractor.extract(&sections, &Artifact::from_file("../../tests/fixtures/rules/profile.yaml")?)?;
```

`Engine::new(LLM::default())` 同理；`engine.judge(判据, items)` 结构化进出——
输出必须是 `{decision, reason}`，缺 reason 视为错误，不让「缺失」看起来正常。

测试与演示通过 `LLM::with_client` 注入客户端，不发网络请求：
`semantic_llm` 按判据回 JSON（覆盖 LLM 首选路径），`demo_llm` 回纯文本（覆盖规则降级路径）。

## 规则与工作流 YAML

与 Dart 包共用同一份资产（toolkit 根的 `tests/fixtures/`，同一件事只写一处）：

```rust
let rule = quanttide_founder::Artifact::from_file("../../tests/fixtures/rules/profile.yaml")?;
let workflow = quanttide_founder::Workflow::from_file("../../tests/fixtures/workflows/classify.yaml")?;
```

判据写在规则里：类别归谁由 `criteria`（意图/判据/正例/反例/易混例）说了算，
换一份 YAML 就换一种解读方式，代码不动。

## 开发

```sh
cargo build
cargo test
cargo fmt --check
cargo clippy

# 解析：把 Markdown 仓库变成语义模型
cargo run --example parse_memory        # memory 仓库解析报告
cargo run --example parse_fiction       # fiction 仓库解析报告

# 工作流：在语义模型上跑计算
cargo run --example memory_workflow     # 四步流程：扫描→分类→分级→合并
cargo run --example fiction_workflow    # 三步提炼→编号轴→阶段流转→包装文案

# 订阅：状态流推给上层（进度 / 流水 / 闸门提示各自订阅）
cargo run --example agent_subscribe
```

示例默认定位主仓库 `assets/memory`、`assets/fiction`，也可跟参数指定路径。

## 许可

Apache License 2.0。
