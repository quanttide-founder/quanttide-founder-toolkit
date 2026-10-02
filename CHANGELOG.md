# Changelog

各包的版本记录在各自目录的 CHANGELOG.md：

- `packages/dart/CHANGELOG.md` — Dart 包
- `packages/rust/CHANGELOG.md` — Rust 包

本文件只记仓库级发布；包内细节看上表。

## [0.1.0-alpha.1] - 2026-09-27

仓库首个 alpha 发布。定位：创始人第二大脑的读写与计算工具箱——把靠目录命名约定组织的 Markdown 文档读成语义模型，人在里面写（日志、小说、洞察），AI 在上面算（提炼、建模、续写）。本版本完成读写基础（解析、规则、编排），计算能力是后续方向。

### Added

- **README**：面向用户重写——能力、语言包、快速开始、文档地图、目录与开发命令
- **docs/**：总述与三套文档成体系——总述讲框架定位与解析三步，用户指南讲装包到跑通流水线，开发者指南讲架构分层、状态机设计与发布流程，API 参考讲 core / memory / fiction 公开接口；代码按 Dart、Rust 分标签切换，附 `myst.yml` 目录
- **packages/dart → dart/v0.1.0-alpha.1**：`quanttide_founder` 首发，core（解析、规则、引擎）+ memory / fiction 两域 + 四个示例 + 行为测试
- **packages/rust → rust/v0.1.0-alpha.1**：`quanttide-founder` 首发，与 Dart 包同构；LLM 调用用 quanttide-agent 的 `LLM`，域编排用 statig 状态机，另附订阅状态流示例

### 内容总览

- `docs/`：总述 + 用户指南（5 篇）+ 开发者指南（4 篇）+ API 参考（4 篇）
- `packages/`：Dart 与 Rust 两个语言包，规则与工作流 YAML 共用 toolkit 根的 `tests/fixtures/`
- `apps/`、`assets/`、`examples/`：子模块，不在本仓库记录版本

### 破坏性变更与迁移指南

无。首个版本，没有既有读者需要迁移。

### Removed

无。
