# 解析引擎示例

本目录包含 memory 和 fiction 两个解析引擎的演示入口。

## 运行

```sh
# memory：解析记忆仓库（journal/profile/insight/roadmap）
dart run example/parse_memory.dart            # 默认定位主仓库 assets/memory
dart run example/parse_memory.dart <路径>      # 指定记忆仓库根目录

# fiction：解析小说仓库（小说/阶段/章节/观察站）
dart run example/parse_fiction.dart            # 默认定位主仓库 assets/fiction
dart run example/parse_fiction.dart <路径>      # 指定小说仓库根目录
```

## memory 解析报告

输出各记忆集的日志时间线、档案、洞察、路线图的结构化统计。

| 记忆层 | 目录 | 模型 | 解析要点 |
|:--|:--|:--|:--|
| 时间线 | `<集>/`、`<集>/journal/` | `JournalEntry` | 文件名即日期；`---` 分隔线切出会话段 |
| 特征 | `<集>/profile/` | `ProfileDoc` | `#` 标题、`##` 主题节、`###` 子节三层 |
| 认知 | `<集>/insight/` | `InsightDoc` | `## 已确认`/`## 假说` 分级；`- **命题**：…依据：…` 解析为条目 |
| 方向 | `<集>/roadmap/` | `RoadmapDoc` | 目标（含 `### 元目标`）、核心问题、已决策、待决策 |

## fiction 解析报告

输出小说列表、各创作阶段的章节统计（编号/字数/预留空号）、观察站素材。

| 层 | 目录 | 模型 | 解析要点 |
|:--|:--|:--|:--|
| 小说 | `{小说名}/` | `Novel` | index.md 为晋江资料；阶段目录按 `{N}_{名称}` 动态发现 |
| 阶段 | `{N}_{阶段}/` | `Stage` | 各小说阶段名不同（灵感/素材/场景/提纲/初稿/改稿/定稿/成稿） |
| 章节 | `{序号}_{标题}.md` | `Chapter` | 编号轴跨阶段共用；预留空号（宁空勿移）；未编号为替代草稿 |
| 观察站 | `观察站/` | `Observation` | 情绪日记（母题发现来源）+ 社会观察 |

## 设计

引擎分三层，自下而上：

1. 通用 Markdown 解析：`MarkdownDocument` 把文本拆成行级块序列（标题、条目、段落、分隔线），不关心语义；
2. 结构切分：`splitSections` 按标题层级切节；
3. 语义提取：规则文件说明「怎么读」，LLM 按规则填表（`semantic_extractor.dart`）。

### 解析规则

**memory**：
- 记忆集发现：根目录下含 `journal/profile/insight/roadmap` 任一子目录的目录即为一个记忆集，点目录跳过；
- 当天日志放集根，与 `journal/` 历史日志统一按文件名日期装载，来源标记区分；
- 缺失结构按可选处理：无假说节的洞察、无元目标的路线图、无一级标题的文件（以文件名兜底标题）均可解析；
- 条目统一走 `parseNamedItem`：`- **名称**：详情` 优先，次选首个冒号切分，纯文本整句作名称。

**fiction**：
- 小说发现：含 `index.md` 或 `N_` 阶段子目录的目录视为小说；
- 阶段动态发现：按 `{N}_{名称}` 模式识别，不硬编码阶段名；
- 章节编号语义：`0_` 前缀为前言（不占正文章节号）；未编号文件为替代草稿；同一序号可有多份（初稿/改稿/定稿各一份）；
- 观察站：`1_情绪日记` 与 `2_社会观察`，跳过 README。

## 文件

| 文件 | 说明 |
|---|---|
| `lib/src/markdown_parser.dart` | 第一层：语法解析（通用） |
| `lib/src/section_splitter.dart` | 第二层：结构切分（通用） |
| `lib/src/semantic_extractor.dart` | 第三层：规则 + LLM（通用） |
| `lib/src/memory_engine.dart` | memory 域模型与装载 |
| `lib/src/fiction_engine.dart` | fiction 域模型与装载 |
| `parse_memory.dart` | memory 演示入口 |
| `parse_fiction.dart` | fiction 演示入口 |
