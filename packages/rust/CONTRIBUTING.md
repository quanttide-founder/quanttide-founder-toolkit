# 贡献指南

仓库级规范（工作原则、资产单份、行为同批、提交规范）见 [toolkit 开发者指南](../../docs/dev-guide/contributing.md)，本文件只记本包的开发操作与本地约定。

## 开发

```sh
cargo build
cargo test
cargo fmt --check
cargo clippy
```

提交前四条全绿；`clippy` 只允许依赖自身的 future-incompat 提示，本包告警视为未通过。

## 测试

五组行为钉子：`engine` / `fiction` / `memory` / `parse` / `rules`，另有 lib 单测。语义行为两条路径都要钉住：

- `semantic_llm`——按提示词里的判据标记回 JSON，覆盖「LLM 首选」路径；
- `demo_llm`——回纯文本，判断必然失败，覆盖「规则降级」路径。

两个客户端都注入 `LLM::with_client`，测试不发网络请求。

## 示例

```sh
cargo run --example parse_memory        # memory 仓库解析报告
cargo run --example parse_fiction       # fiction 解析报告
cargo run --example memory_workflow     # 四步流程：扫描→分类→分级→合并
cargo run --example fiction_workflow    # 三步提炼→编号轴→阶段流转→包装文案
cargo run --example agent_subscribe     # 状态流推给上层
```

示例默认定位主仓库 `assets/memory`、`assets/fiction`，也可跟参数指定路径。两个工作流示例在配置 `LLM_API_KEY` 时走真实判断（`memory_workflow` 演示只判前 10 行，全量逐行以分钟计），未配置则用演示客户端走显式降级，离线可跑。

## 资产与对齐

- 规则与工作流 YAML 只在 toolkit 根 `tests/fixtures/`，本包编译期嵌入（`src/{memory,fiction}/rules.rs`），另一侧引用，不复制；
- 与 Dart 的行为改动同批完成、测试同批补；结构性差异登记在 [CHANGELOG.md](CHANGELOG.md) 的「与 Dart 包的差异」，不口头约定。

## 提交与发布

提交信息用 Conventional Commits（`feat` / `fix` / `docs` / `test` / `refactor` / `chore`），单次提交独立完整、验证后再提交。

发版步骤：

1. bump `Cargo.toml` 版本与 README 安装片段，`Cargo.lock` 同步；
2. [CHANGELOG.md](CHANGELOG.md) 把 `[Unreleased]` 收成新版本段；
3. `cargo test` / `fmt` / `clippy` 全绿后提交推送；
4. `qtcloud-devops release publish --version rust/vX.Y.Z`，再回主仓库更新子模块引用。
