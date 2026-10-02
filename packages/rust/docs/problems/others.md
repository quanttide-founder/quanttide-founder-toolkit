同类的「套概念」问题

先给判据——怎么识别一个概念是「套」上去的：

它有一个名字，这个名子暗示了一种意图，但代码里没有对应的行为。或者更糟——行为属于另一个概念。

按这个判据扫一遍，找到三处。

—

① 标签（Tag）

alpha2 新加的：

```rust
impl JournalEntry {
    pub fn tags(&self) -> Vec<Tag> {
        vec![
            Tag::new(”date“, self.date.to_string()),
            Tag::new(”source“, ...),
        ]
    }
}
```

这是你的三分法里的「tag」——AI 把它实现了。

但日期和来源不是 tag。

· tag 是用户定义的、语义性的——「关于写作」「关于产品」
· date / source 是系统元数据——从文件名和路径来的

AI 听到你说「tag 是 KV」，就把任何 KV 都叫 tag 了。 它没理解 tag 在你的分类体系里是和 category 并列的一个语义维度。

它把「元数据」和「标签」合并了。 就像它把「替代方案」和「失败兜底」合并成了「降级」。

同一个操作：用一个词吞掉两个不同的东西。

—

② 候选（Candidate）

Engine::run 里：

```rust
Verb::Scan => {
    let keywords = keywords_from_rules(&workflow.rules);
    candidates.extend(Self::scan(&items, &keywords));
}
Verb::Judge => {
    let judgment = self.judge(&workflow.rules, &items)?;   // ← 用的是 items，不是 candidates
    judgments.push(judgment);
}
```

candidates 被算出来了，被存起来了，然后——没人用。

Judge 分支用的是 items，不是 candidates。candidates 只是被塞进 WorkflowResult 里作为输出。

「候选」这个词暗示：这是经过筛选、准备进入下一步的东西。

但实际是：它被生产出来，然后被忽略。

对比 MemoryFlow 的 Located 状态——那里的「候选」是真的被 Route 消费的。同一个词，两种行为。

「候选」是一个被套上去的概念——它在一个地方是真的，在另一个地方是装饰。

—

③ 动作（Action）

Workflow.actions：

```rust
pub actions: Vec<Action>,
```

Action 是 YAML 里定义的「可选决策」——add / rewrite / replace / crossLink。

但实际合并逻辑是硬编码的：

```rust
match decision.action.as_str() {
    ”add“ => result.push(decision.target),
    ”rewrite“ | ”replace“ => { ... }
    _ => {}
}
```

动作名来自 YAML，但动作的行为在代码里。

YAML 里能改的是名字——add 改成 insert——但改完之后，match 里的 ”add“ 就不认识了，会掉到 _ 分支。

「动作」这个概念暗示：YAML 定义动作，代码执行动作。

实际是：代码定义四个动作，YAML 只能给它们改名。

这又是一个套概念——把「枚举改名」包装成了「数据驱动」。

—

共同点

三处都是：

· 有一个名字：Tag / Candidate / Action
· 这个名字在你（设计者）的语汇里有意义
· 代码没有实现那个意义
· 或者实现了另一个东西，套了同一个名

和「降级」完全一样的模式。 AI 从代码里归纳出一个名字，然后把这个名字当成设计意图。

—

为什么这些比「降级」轻

· 降级：直接让错误数据流入主路径——有实际后果
· Tag / Candidate / Action：是「概念虚化」——没有直接后果，但让你的设计语汇和代码脱节

你未来看代码时，会以为 tags() 返回的是你分类体系里的 tag。 但它不是。这会误导你自己。

—

一句话

同一个模式：AI 从代码里归纳出一个名字，然后把这个名字当成你的设计意图。

Tag 吞掉了「元数据和标签」。Candidate 吞掉了「筛选和收集」。Action 吞掉了「数据驱动和枚举改名」。

你未来会误以为它们做的是你设计的事。
