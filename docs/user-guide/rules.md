# 规则与工作流

规则是数据不是代码：一种文档怎么读、什么时候归它，写在规则文件里；一个任务怎么做，写在工作流文件里。改写法只改 YAML，不改引擎。

## 规则文件

规则文件（Artifact）描述一种文档类型的读法：文档类型、标题来源、说明位置、章节切分与提取字段，外加两块数据——`criteria`（归类判据：意图、判据、正例、反例、易混例）与 `grades`（分级类别）。以 `tests/fixtures/rules/profile.yaml` 为例：

~~~yaml
name: profile            # 类别 key——去向分类输出的就是这个名字
document_type: profile

criteria:
  intent: 我是什么样的人——稳定特征
  rules: "文本在说写作者的一贯特征……"
  positive: [我总是把截止日当起点]
  negative: [今天我很烦躁（这是一次情绪）]
  ambiguous: [「我这次又拖延了」——偶发事件则留 journal，指向一贯模式则归 profile]

title:
  source: first_h1       # 固定取值，加载期校验
  fallback: filename

description:
  location: preface      # 固定取值：H1 之后、首个 H2 之前的段落
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

`instruction()` 把读法转成自然语言描述，`criteria_instruction()` 把判据转成判断提示，都供 LLM 用；LLM 按规则填表与判断，不自由发挥。`split_by` / `source` / `location` 是固定取值（H1-H6、first_h1 等），`split_by: h7` 这类写法加载即失败。

## 加载

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~dart
final rule = Artifact.fromFile('tests/fixtures/rules/profile.yaml');
final workflow = Workflow.fromFile('tests/fixtures/workflows/classify.yaml');
print(rule.instruction);
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~rust
let rule = Artifact::from_file("tests/fixtures/rules/profile.yaml")?;
let workflow = Workflow::from_file("tests/fixtures/workflows/classify.yaml")?;
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

步骤写成单键映射，键是动词、值是步骤描述；出现三个动词之外的键，加载时报错。`scan` 的关键词表从 `rules` 文本里读（含「关键词」的行），关键词是数据，代码不持有关键词表。执行入口与各步语义见[编排与状态流](orchestration.md)。

## 资产位置

两包共用同一份 YAML（toolkit 根的 `tests/fixtures/`），不各存一份。规则按类型各一份（profile / insight / roadmap / intention / observation / fragment / packaging），工作流按 classify、grade、cluster、locate 四个任务各一份。
