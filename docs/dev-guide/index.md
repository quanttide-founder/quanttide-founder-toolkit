# 开发者指南

这组文档面向维护工具箱代码的人：仓库怎么分层、状态机为什么长这样、两包怎么保持一致、版本怎么发。

## 页面导航

- [状态机设计](state-machine.md)：bloc 换成 statig 的理由、状态与事件怎么加、订阅的边界
- [开发与测试](contributing.md)：工具链、测试布局、两包一致性纪律
- [版本与发布](release.md)：版本契约、CHANGELOG 分工、scope 标签发布

## 仓库结构

```text
quanttide-founder-toolkit/
├── docs/                    文档（本组与总述）
├── packages/
│   ├── dart/                quanttide_founder（Dart 实现）
│   │   ├── lib/src/         core/ memory/ fiction/
│   │   ├── example/         解析与工作流四入口
│   │   ├── test/            行为钉子
│   │   └── assets/          规则与工作流 YAML（两包共用）
│   └── rust/                quanttide-founder（Rust 实现）
│       ├── src/             core/ memory/ fiction/ + error
│       ├── examples/        四入口 + agent_subscribe
│       └── tests/           行为钉子
└── CHANGELOG.md             各包版本记录的索引
```

## 分层职责

两个域共用一套解析，只换规则文件；每一层回答一个问题，越界即失职。

| 层 | 回答 | 不做什么 |
| :-- | :-- | :-- |
| `core/parse` | 文本长什么样 | 不认业务 |
| `core/rules` | 名词和任务怎么定义 | 不执行 |
| `core/engine` | 怎么加工（scan / judge / merge） | 不管结构发现 |
| `{域}/models` | 有什么 | 不读文件（Dart 的 `parse(File)` 是历史遗留，Rust 侧已改为收字符串） |
| `{域}/repository` | 从哪装载 | 不做语义 |
| `{域}/states` | 做成什么 | 不管 I/O |

## 四条纪律

- core 不依赖域，域依赖 core；跨域共享只进 `core/`
- 规则是数据：文档怎么读写在 YAML 里，引擎只解释规则，改写法不改代码
- LLM 不自带客户端：Dart 侧是 `LlmClient` 接口由使用方实现，Rust 侧直接用 quanttide-agent 的 `LLM`
- 两包行为对齐：同一批行为同改，取舍不同的地方登记在 `packages/rust/CHANGELOG.md` 的「与 Dart 包的差异」
