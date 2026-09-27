# 示例

本目录演示工具箱的两组能力：**解析**（读 Markdown 仓库）和**工作流**（在语义模型上计算）。

## 运行

```sh
# 解析：把 Markdown 仓库变成语义模型
dart run example/parse_memory.dart        # memory 仓库解析报告
dart run example/parse_fiction.dart       # fiction 仓库解析报告

# 工作流：在语义模型上跑计算
dart run example/memory_workflow.dart     # MemoryBloc：扫描→分类→分级→合并
dart run example/fiction_workflow.dart    # FictionBloc：三步提炼→编号轴→阶段流转→包装文案
```

## 解析示例

输出仓库的结构化统计：

- **parse_memory.dart**：各记忆集的日志时间线、档案、洞察、路线图
- **parse_fiction.dart**：小说列表、各阶段章节（编号/字数/预留空号）、观察站素材

## 工作流示例

在解析结果上跑业务逻辑：

- **memory_workflow.dart**：MemoryBloc 的四步流程——扫描（关键词定位）、分类（三问路由）、分级（证据判断）、合并（四种策略）
- **fiction_workflow.dart**：FictionBloc——三步提炼（取样→观察展开→落实片段）、章节编号轴（填空号/追加）、阶段流转（完成度检查）、包装文案提取

## 文件

| 文件 | 演示 |
|---|---|
| `parse_memory.dart` | MemoryRepository 装载 + 解析报告 |
| `parse_fiction.dart` | FictionRepository 装载 + 解析报告 |
| `memory_workflow.dart` | MemoryBloc：扫描/分类/分级/合并 |
| `fiction_workflow.dart` | FictionBloc：提炼/编号轴/阶段流转/包装 |
