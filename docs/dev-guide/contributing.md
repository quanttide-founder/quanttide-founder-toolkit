# 开发与测试

改任何一侧，先跑通两侧的门禁；行为有出入，先改代码再登记差异，不许各自漂移。

## 工具链

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~sh
cd packages/dart
dart pub get
dart analyze
dart format --set-exit-if-changed --output=none .
dart test
dart run example/parse_memory.dart
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~sh
cd packages/rust
cargo build
cargo fmt --check
cargo clippy --all-targets
cargo test
cargo run --example parse_memory
~~~

:::
```

## 测试与示例布局

- Dart：行为钉子在 `test/`，示例在 `example/`，测试不依赖文件系统的用内存内容
- Rust：按对象分文件在 `tests/`（`parse`、`rules`、`engine`、`memory`、`fiction`），共用夹具在 `tests/common/`，临时仓库用 `tempfile` 现场搭
- 两侧的示例同一套名字：`parse_memory`、`parse_fiction`、`memory_workflow`、`fiction_workflow`，对真实 `assets/` 跑出的报告应当一致
- Rust 另有 `agent_subscribe`，演示订阅状态流

## 两包一致性

1. 行为同改：解析、路由、分级、合并、编号轴这些语义改动必须两侧同批完成，测试同批补
2. 命名对应：类型名两侧一致（`MemorySet`、`InsightItem`），方法名 Dart 驼峰、Rust 蛇形（`decideMerge` 对 `decide_merge`）
3. 差异登记：结构性取舍不同（LLM 客户端、状态机、错误通道）写进 `packages/rust/CHANGELOG.md` 的「与 Dart 包的差异」，不口头约定
4. 资产单份：规则与工作流 YAML 只在 `packages/dart/assets/`，另一侧引用，不复制

## 提交

提交信息用 Conventional Commits（`feat` / `fix` / `docs` / `test` / `refactor` / `chore`），单次提交独立完整、验证后再提交；子模块操作前先回到 main。仓库级规范见主仓库 `AGENTS.md`。
