# 量潮创始人工具箱（quanttide-founder-toolkit）

创始人第二大脑的读写与计算工具箱：把靠目录命名约定组织的 Markdown 文档读成语义模型——人在里面写（日志、小说、洞察），AI 在上面算（提炼、建模、续写）。Dart 与 Rust 双实现，行为对齐。

本仓库是 [quanttide-founder](https://github.com/quanttide-founder/quanttide-founder) 的子模块。

## 能做什么

- 解析：装载 memory 与 fiction 两类仓库，产出日志、档案、洞察、路线图、小说、观察站模型；没有二级标题、没有一级标题、章节未编号都照常解析
- 规则：一种文档怎么读，写在 YAML 规则文件里——改写法只改规则，不改引擎；LLM 按规则提取字段，不自由发挥
- 编排：扫描、判断、合并三个动词组装任务——memory 四步流程（扫描 → 过滤 → 聚类 → 合并），fiction 三步提炼（取样 → 观察展开 → 落实片段）
- 计算：分类、证据分级、聚类、合并、升降级、候选定位六类计算的输入已备好，计算本身是后续方向

## 语言包

| 包 | 语言 | 版本 | 获取方式 |
| :-- | :-- | :-- | :-- |
| `packages/dart` | Dart | `dart/v0.1.0-alpha.1` | GitHub Release，尚未上架 pub.dev |
| `packages/rust` | Rust | `rust/v0.1.0-alpha.1` | GitHub Release，尚未上架 crates.io |

两侧同构：类型名一致，方法名各随语言惯例（`decideMerge` 对 `decide_merge`）；结构性取舍差异登记在 `packages/rust/CHANGELOG.md`。

## 快速开始

克隆主仓库后直接跑示例，示例默认定位 `assets/memory` 与 `assets/fiction`：

```sh
# Dart
cd packages/dart
dart pub get
dart run example/parse_memory.dart

# Rust
cd packages/rust
cargo run --example parse_memory
```

安装进你自己的项目、读懂输出、改规则文件，见[用户指南](docs/user-guide/index.md)。

## 文档

- [总述](docs/index.md)：框架定位、解析三步、memory 与 fiction 的共同结构
- [用户指南](docs/user-guide/index.md)：快速开始、解析、规则与工作流、编排与状态流
- [开发者指南](docs/dev-guide/index.md)：架构分层、状态机设计、开发测试、版本发布
- [API 参考](docs/api-references/index.md)：core / memory / fiction 公开接口

文档内代码按 Dart、Rust 分标签切换；站点构建见 [docs/myst.yml](docs/myst.yml)。

## 目录

```text
quanttide-founder-toolkit/
├── docs/              文档（总述 + 三套文档）
├── packages/
│   ├── dart/          Dart 包 quanttide_founder
│   └── rust/          Rust 包 quanttide-founder
├── CHANGELOG.md       仓库级发布记录与各包索引
└── README.md          本文件
```

## 开发

```sh
# Dart
cd packages/dart && dart pub get && dart analyze && dart test

# Rust
cd packages/rust && cargo test && cargo fmt --check && cargo clippy
```

提交与发布规范见主仓库 `AGENTS.md`；版本记录在各包 `CHANGELOG.md`。

## 许可

Apache License 2.0，详见 [LICENSE](LICENSE)。
