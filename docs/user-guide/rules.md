# 规则与工作流

规则是数据不是代码：一种文档怎么读，写在规则文件里；一个任务怎么做，写在工作流文件里。改写法只改 YAML，不改引擎。

## 规则文件

规则文件（Artifact）描述一种文档类型的读法，四个部分：文档类型、标题来源、说明位置、章节切分与提取字段。以 `assets/rules/profile.yaml` 为例：

~~~yaml
document_type: profile

title:
  source: "第一个 H1 标题"
  fallback: "文件名去掉 .md"

description:
  location: "H1 之后、首个 H2 之前的段落"
  meaning: "来源说明，描述这份档案从哪来、覆盖什么"

sections:
  split_by: H2
  subsections: H3
  extract:
    - field: items
      from: "节内列表条目"
      patterns:
        - "**名称**：详情"
        - "名称：详情"
  unknown_content: "保留在 paragraphs 中，不丢弃"
~~~

`instruction()` 把规则转成自然语言描述，供 LLM 理解「怎么读」；LLM 按规则填表，不自由发挥。

## 加载

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~dart
final rule = Artifact.fromFile('assets/rules/profile.yaml');
final workflow = Workflow.fromFile('assets/workflows/classify.yaml');
print(rule.instruction);
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~rust
let rule = Artifact::from_file("assets/rules/profile.yaml")?;
let workflow = Workflow::from_file("assets/workflows/classify.yaml")?;
println!("{}", rule.instruction());
~~~

:::
```

## 工作流与三个动词

工作流（Workflow）由步骤组成，每步是一个动词加一句描述。动词只有三个：

- `scan`：按关键词定位候选，代码做，确定性
- `judge`：让 LLM 按 `rules` 判断，拿不准不收
- `merge`：按策略合并条目，代码做

~~~yaml
name: classify
description: 三问过滤——判断日志条目的去向
input: journal
output: [insight, profile, roadmap, journal]
steps:
  - scan: 按认知关键词定位候选
  - judge: 三问判断去向，拿不准不收
rules: |
  「想通了什么」→ insight、「我是谁」→ profile、
  「要做什么」→ roadmap、「发生了什么」→ 留 journal。
~~~

步骤写成单键映射，键是动词、值是步骤描述；出现三个动词之外的键，执行时报错。执行入口与各步语义见[编排与状态流](orchestration.md)。

## 资产位置

两包共用同一份 YAML（`packages/dart/assets/`），不各存一份。仓库级规则的扩展顺序是 profile 之后补 insight、roadmap；工作流按 classify、grade、cluster、locate 四个任务各一份。
