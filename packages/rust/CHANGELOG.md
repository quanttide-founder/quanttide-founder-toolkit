# Changelog

## [0.1.0-alpha.1] - 2026-09-27

首个 alpha 预发布，行为对齐同日发布的 Dart 包 `quanttide_founder` 0.1.0-alpha.1。

### Added

- **core/**：parse（Markdown 解析 + 结构切分）、rules（Artifact / Workflow YAML 加载）、engine（scan / judge / merge + 规则/LLM 两个提取器）
- **memory/**：models（JournalEntry / InsightDoc / ProfileDoc / RoadmapDoc）、repository（MemoryRepository）、states（四步流程状态机 + 三问路由、证据分级、聚类、合并策略、升降级）
- **fiction/**：models（Novel / Chapter / Stage / Observation）、repository（FictionRepository）、states（三步提炼状态机 + 章节编号轴、阶段流转、包装文案）
- **example/**：解析 + 工作流四入口（parse_memory / parse_fiction / memory_workflow / fiction_workflow）+ 订阅演示（agent_subscribe：Agent 驱动任务状态机，进度/流水/闸门提示各自订阅状态流，`after_transition` 钩子推快照）
- **tests/**：parse / rules / engine / memory / fiction 五组行为钉子

### 与 Dart 包的差异

- **LLM**：不复刻 `LlmClient` 接口，改用 quanttide-agent 的 `LLM`——`Engine::judge`、`LlmExtractor` 直接用它发请求，响应 JSON 剥壳走 `quanttide_agent::parse_structured_output`
- **编排**：Dart 的 bloc 换成 statig 状态机，模块名 `memory/states.rs`、`fiction/states.rs`；四步/三步流程是事件驱动的状态转移（`Scan/Route/Cluster`、`Sample/Expand/Settle`），判断规则收成自由函数
- **流程参数**：`MemoryFlow::process_journal` 不再收 Dart 侧未参与计算的 `Workflow` 参数，流程定义即状态机本身
- **日期正则**：日志文件名匹配用行尾锚定 `\.md$`——Dart 侧 `RegExp(r'...\.md\$')` 把 `$` 写成了字面量，永远匹配不到 `YYYY-MM-DD.md`，`MemoryRepository.load` 在 Dart 侧读不出日志（待修）
