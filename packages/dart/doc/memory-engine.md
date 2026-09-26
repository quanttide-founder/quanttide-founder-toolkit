# memory 解析引擎

引擎分三层：通用 Markdown 解析把文本拆成行级块序列；类型化解析在块序列上按标题层级切节并提取各层语义；仓库装载发现记忆集并逐层组装为 `MemorySet`。

## 使用方式

导入包入口后即可使用：

```dart
import 'package:quanttide_founder/quanttide_founder.dart';

void main() {
  final repo = MemoryRepository.load('path/to/assets/memory');
  for (final set in repo.sets) {
    print('${set.name}: ${set.journals.length} 篇日志');
  }

  final insight = repo.sets.first.insights.first;
  for (final item in insight.itemsOf(InsightGrade.confirmed)) {
    print('${item.statement} — ${item.evidence}');
  }

  final roadmap = repo.sets.first.roadmaps.first;
  print(roadmap.pending.map((item) => item.name));
}
```

仓库根目录不存在时 `load` 抛出 `FileSystemException`。低层接口 `MarkdownDocument.parse` 与 `splitSections` 也随包导出，可单独解析任意 Markdown 文件。

演示入口输出解析报告：

```sh
dart run example/parse_memory.dart            # 默认定位主仓库 assets/memory
dart run example/parse_memory.dart <路径>      # 指定记忆仓库根目录
```

## 解析规则

### 结构映射

| **记忆层** | **目录** | **模型** | **解析要点** |
|:--|:--|:--|:--|
| 时间线 | `<集>/`、`<集>/journal/` | `JournalEntry` | 文件名即日期；`---` 分隔线切出会话段 |
| 特征 | `<集>/profile/` | `ProfileDoc` | `#` 标题、`##` 主题节、`###` 子节三层 |
| 认知 | `<集>/insight/` | `InsightDoc` | 「已确认」「假说」二级标题分级，条目按命题、解释、依据拆解 |
| 方向 | `<集>/roadmap/` | `RoadmapDoc` | 目标、元目标、核心问题、已决策、待决策五段结构 |

### 通用规则

- 记忆集发现：根目录下含 `journal/profile/insight/roadmap` 任一子目录的目录即为一个记忆集，点开头的目录跳过。
- 只读取 `.md` 文件并跳过 `README.md`；日志仅收 `YYYY-MM-DD.md` 命名的文件。
- 集根日期文件与 `journal/` 历史日志统一按日期倒序，以 `JournalSource.root/archive` 区分来源。
- 结构一律按可选处理：缺假说节的洞察、缺元目标的路线图、缺一级标题的文件均可解析，标题以 `name` 字段（文件名去扩展名）兜底。
- 未知结构不丢内容：路线图中非方向层的二级标题落入 `themeSections`，档案与洞察的其余节按普通节保留。

### 分层规则

时间线：`---`、`----`、`--` 等纯横线行视为会话分隔，切分出的非空段落为 `segments`；`characterCount` 忽略所有空白后计字数。

特征：一级标题是 `title`，其后、首个二级标题之前的段落是 `description`（来源说明）；二级标题切主题节，三级标题切子节。

认知：标题含「已确认」的节映射为 `InsightGrade.confirmed`，含「假说」的映射为 `InsightGrade.hypothesis`，其余节 `grade` 为空（主题式散文）。条目文本形如 `- **命题**：解释。依据：证据`，拆为 `statement`、`detail`、`evidence` 三个字段，`detail` 中不含依据。

方向：`## 目标` 的正文段落是 `goal`，其下 `### 元目标` 的条目归入 `metaGoals`；`## 核心问题`、`## 已决策`、`## 待决策` 的条目分别归入同名字段。条目统一经 `parseNamedItem` 解析：`- **名称**：详情` 优先匹配，其次取首个全角或半角冒号切分，无冒号时整句作名称、详情为空。
