# 量潮创始人工具箱（quanttide-founder-toolkit）

创始人工具箱——面向创始人角色的事务与工具集合。

## 这个库是做什么的

给「第二大脑」建一个**读懂层**——让 AI 能理解结构化知识的语义，而不只是识别 Markdown 语法。

核心能力是 **memory 解析引擎**：把记忆仓库的四层结构（journal / profile / insight / roadmap）解析成机器可读的语义模型。不管公有还是私有、不管 memory 还是 fiction，都能用同一套规则 + LLM 去理解，在此之上做业务域建模、做提炼、做产品。

## 怎么理解三层管线

| 层 | 做什么 | 怎么做 |
|---|---|---|
| 第一层 | 语法解析 | 正则识别 Markdown 的标题、段落、列表、分隔线 |
| 第二层 | 结构切分 | 按标题层级（H1/H2/H3）切节 |
| 第三层 | 语义提取 | 规则文件说明「怎么读」，LLM 按规则填表 |

关键设计：**正则只管结构，语义交给规则 + LLM**。规则是 YAML 数据不是代码——改写法只改规则文件，不改引擎。LLM 按规则填表，不自由发挥。

## 包结构

```
packages/dart/
├── assets/rules/          解析规则（profile.yaml 等，逐步扩展）
├── lib/src/
│   ├── memory_engine.dart       第一层 + 第二层（语法解析 + 结构切分）
│   └── semantic_extractor.dart  第三层（规则 + LLM 语义提取）
├── doc/                   引擎文档
├── example/               演示入口
└── test/                  测试
```

## 解析规则

规则文件描述一种文档类型怎么读：标题从哪来、说明在哪、章节怎么切、提取哪些字段。以 `assets/rules/profile.yaml` 为例：

- 标题取第一个 H1，兜底用文件名
- 说明是 H1 与首个 H2 之间的段落
- 按 H2 切节、H3 切子节
- 提取 name / paragraphs / items 三个字段
- 未知内容保留在 paragraphs，不丢弃

后续扩展 `insight.yaml`、`roadmap.yaml`，每种文档类型一份规则。

## 提取器可插拔

- **RuleBasedExtractor**：纯规则匹配，无 LLM 时的降级方案
- **LlmExtractor**：LLM 按规则填表，首选方案，需提供 LlmClient 实现

## 开发

```sh
cd packages/dart
dart pub get
dart analyze
dart test
```
