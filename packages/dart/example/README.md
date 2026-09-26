# memory 解析引擎示例

本目录包含解析 `assets/memory` 记忆仓库的 Dart 引擎。引擎按仓库当前结构（记忆集 × 四层）建模，容忍各层的实际格式变体。

## 运行

```sh
dart run example/parse_memory.dart            # 默认定位主仓库 assets/memory
dart run example/parse_memory.dart <路径>      # 指定记忆仓库根目录
```

运行后输出解析报告：各记忆集的日志时间线、档案、洞察、路线图的结构化统计。

## 设计

引擎分三层，自下而上：

1. 通用 Markdown 解析：`MarkdownDocument` 把文本拆成行级块序列（标题、条目、段落、分隔线），不关心语义；
2. 类型化解析：`splitSections` 按标题层级切节，四个解析器在其上提取各层语义模型；
3. 仓库装载：`MemoryRepository` 发现记忆集并逐层装载为 `MemorySet`。

### 结构映射

| 记忆层 | 目录 | 模型 | 解析要点 |
|:--|:--|:--|:--|
| 时间线 | `<集>/`、`<集>/journal/` | `JournalEntry` | 文件名即日期；`---` 分隔线切出会话段 |
| 特征 | `<集>/profile/` | `ProfileDoc` | `#` 标题、`##` 主题节、`###` 子节三层 |
| 认知 | `<集>/insight/` | `InsightDoc` | `## 已确认`/`## 假说` 分级；`- **命题**：…依据：…` 解析为条目 |
| 方向 | `<集>/roadmap/` | `RoadmapDoc` | 目标（含 `### 元目标`）、核心问题、已决策、待决策 |

### 解析规则

- 记忆集发现：根目录下含 `journal/profile/insight/roadmap` 任一子目录的目录即为一个记忆集，点目录跳过；
- 当天日志放集根，与 `journal/` 历史日志统一按文件名日期装载，来源标记区分；
- 缺失结构按可选处理：无假说节的洞察、无元目标的路线图、无一级标题的文件（以文件名兜底标题）均可解析；
- 写作集的路线图是主题式而非方向层五段结构，未知的二级标题落入 `themeSections`，不丢内容；
- 条目统一走 `parseNamedItem`：`- **名称**：详情` 优先，次选首个冒号切分，纯文本整句作名称。

## 文件

- `lib/src/memory_engine.dart`：引擎本体（模型与解析器），由 `package:quanttide_founder/quanttide_founder.dart` 导出；
- `parse_memory.dart`：演示入口，输出解析报告。
