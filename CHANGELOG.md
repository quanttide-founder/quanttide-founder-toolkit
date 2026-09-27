# Changelog

## [0.1.0-alpha.1] - 2026-09-27

首个 alpha 预发布。

### Added

- **core/**：parse（Markdown 解析 + 结构切分）、rules（Artifact / Workflow YAML 加载）、engine（scan / judge / merge + 提取器）、llm（LlmClient 接口）
- **memory/**：models（JournalEntry / InsightDoc / ProfileDoc / RoadmapDoc）、repository（MemoryRepository）、bloc（四步流程、三问路由、证据分级、合并策略）
- **fiction/**：models（Novel / Chapter / Stage / Observation）、repository（FictionRepository）、bloc（三步提炼、章节编号轴、阶段流转、包装文案）
- **assets/**：artifacts/（名词定义 YAML）+ workflows/（动词定义 YAML）骨架
- **example/**：解析 + 工作流四入口（parse_memory / parse_fiction / memory_workflow / fiction_workflow）
