# 设计与执行的错位

## 问题的重新定位

前几轮的分析批评了实现：`route` 里用「要」短路一切、`expand_observation` 原样返回、`Engine::run` 的 `scan` 清空 `items`。这些批评仍然成立，但它们批评的对象是错的。它们针对的是「一个糟糕的实现」，却没有先问一个更基本的问题：这个实现试图实现什么？

把 `route` 的五张关键词表（`COGNITION` / `INTENT` / `DIRECTION` / `PROFILE`）摊开看，这不是随手写的 if-else，而是有人在试图实现一个认知分类模型，用关键词作初步近似。

「设计者是资深专家，AI 是执行」这个前提让一切重新排列。从代码里能读到的，不再是「一个半成品」，而是一份清晰的设计意图，被一份笨拙的执行压住了。下面先看设计者想做什么，再看实现为什么没跟上。

## 业务意图

把 memory 和 fiction 并排看，会看到一个共同结构，它指向同一个定位——认知提炼器：

```text
原始表达  ──提炼──▶  结构化认知
```

- memory：日记 → 洞察 / 画像 / 路线图 / 意图；
- fiction：情绪日记 → 母题 / 场景素材。

这不是「笔记工具」，也不是「写作工具」。这两类工具都有现成的范式（Notion、Obsidian、Scrivener），而这份代码没有采用其中任何一个。它走的是另一条路：从自我表达中提炼出被压缩过的自我认识。

这可以从 memory 的目的地看出来。`Destination` 的五个变体不是随意的：

| **去向** | **回答的问题** | **层次** |
|:--|:--|:--|
| Insight | 我发现了什么？ | 认知 |
| Profile | 我是什么样的人？ | 特质 |
| Roadmap | 我要往哪走？ | 方向 |
| Intention | 我到底想要什么？ | 意愿 |
| Journal | 发生了什么？ | 事件 |

这五个去向合起来，是一套关于「创始人是谁」的认知模型。它关心的不是「怎么找到信息」（那是知识管理），而是「这句话在说我的哪一面」。

这个区分是业务的核心。同一段日记被分到哪个去向，决定了它成为什么样的资产。而「该分到哪」不是内容问题，是关于写作者的判断——这正是需要 LLM 的地方，也是设计者把 `Engine::judge` 放进 core 的原因。

fiction 那侧有一个更精炼的表达：

> 直接从情绪跳到场景会产出「金句」而不是「画面」——不到 `Observed` 就不给 `Settle`。

这一句里藏着一条创作理论：情绪本身不是素材，情绪经过外化观察之后才是素材。设计者用状态机强制这个顺序——`Sample → Expand → Settle`，跳过 `Expand` 就不给 `Settle`。这不是技术约束，是审美约束。一个人不会为了「跳过中间步骤」而设计一个状态机来禁止它，除非他真的相信那个中间步骤不可跳过。这是一个懂创作的人才会写的约束。

## 设计意图

从代码里能读出的设计决策有七条。每一条都不是 AI 会自己发明的——它们是设计者做出的选择。

### core 是语言，不是库

`core/` 提供类型（`Block` / `RawSection`）、错误（`Error`）、概念（`Artifact` / `Workflow`），但不提供「memory 怎么读」「fiction 怎么读」。core 跨域共享，域依赖 core，core 不依赖域。

这条原则在很多地方被违反，比如 `RuleBasedExtractor` 里的「前言段 → description」其实是 memory 的约定，但意图是清楚的：core 定义词汇，域定义句子。

### 模型、仓库、状态三层分离

三个域都是 `models.rs` / `repository.rs` / `states.rs`，这不是巧合，它对应领域驱动设计里的分层：

- `models`：数据长什么样；
- `repository`：数据从哪来；
- `states`：数据怎么变。

### 状态机代替手写流程

域编排不用手写流转，而用 statig 状态机——事件进、状态出，走到哪一步一目了然。

用状态机而不是 if-else 流，代价是学习成本和样板代码，收益是流程本身成为可检查的对象：可以问「从 `Located` 能不能到 `Clustered`」，类型系统会回答。这是「流程即数据」的设计观，不是所有人都愿意付出这个代价。

### 规则外部化

`Artifact` / `Workflow` 是 YAML，与 Dart 共享同一份资产（toolkit 根的 `tests/fixtures/`，同一件事只写一处）。

设计意图是：规则是数据，不是代码。换一份 YAML，就换一种解读方式。这让 memory 和 fiction 能共用同一套解析引擎，只是配置不同。这是一个有远见的选择，但实现层面没人读那份 YAML。

### LLM 是语义算子，不是对话界面

这是整个设计里最不寻常的一条。大多数「AI 应用」把 LLM 当作对话对象（用户提问、模型回答），这份代码不这样。在这里，LLM 的角色是一个纯函数：

```rust
fn judge(rules: &str, items: &[Value]) -> Judgment
fn extract(sections: &[RawSection], rule: &Artifact) -> ExtractionResult
```

输入是结构化数据，输出也是结构化数据。LLM 只承担「理解语义」这一步，其余全由代码完成。

这意味着设计者不认为 LLM 是产品本身，而是把 LLM 放进「计算」这个类别——像一个数据库、一个求解器。这个判断是成熟的：它既防止了「用 LLM 做一切」的典型陷阱，也防止了「LLM 是黑盒」的恐惧。

### 可插拔的提取策略

```rust
pub trait SemanticExtractor {
    fn extract(&self, sections: &[RawSection], rule: &Artifact) -> Result<ExtractionResult, Error>;
}
```

两个实现：

- `LlmExtractor`——首选，用 LLM 填表；
- `RuleBasedExtractor`——降级，纯规则。

这是「首选 + 降级」的双路径设计。设计者知道 LLM 可能不可用，也知道它可能给出错误结果，所以留了一条退路。这同样是成熟判断：不把 LLM 当唯一解，也不把它当可选项。

### 跨语言对齐

与 Dart 包「行为对齐」，并明确列出「两处按 Rust 的做法另选实现」：LLM 调用方式、域编排方式。

这是「语言合适性优先于表面一致性」的判断。不追求两边代码长得一样，而是追求用户看到的行为一样、实现按语言习惯来。设计者经历过跨语言项目，知道「逐行翻译」的陷阱。

## 设计者画像

从代码里能反推出设计者的几个特征。

### 懂业务，而且有理论

`Insight` / `Profile` / `Roadmap` / `Intention` 这套分类不是从别处抄的。它出现在 `Destination`、`InsightGrade`（`Confirmed` / `Hypothesis`）、`MergeAction`（`Add` / `Rewrite` / `Replace` / `CrossLink`）——它是一套完整的知识论，回答「什么样的认知值得保留、怎么保留、什么时候升级」。

尤其 `should_promote_to_profile`：「命题反复套用、稳定为思维框架 → 晋升」。这不是「笔记系统」会有的概念，这是认知发展模型。

### 懂写作，而且有实践

`Packaging` 里的注释写着「标题 = 核心矛盾与钩子；一句话简介 = 增量信息（≤15 字）；立意 = 价值维度」。「≤15 字」「增量信息」是真实的平台约束（晋江），不是编的。

`Chapter` 里的编号规则是「序号对应正文最终阅读顺序，各阶段共用同一编号轴；预留空号（宁空勿移），同一序号可有多份（初稿 / 改稿 / 定稿）」。「宁空勿移」是编辑经验，不是程序员会想到的——程序员想的是「编号要连续」。这条规则说明设计者真的改过稿。

### 懂技术，而且到架构级

- DDD 分层；
- 状态机代替手写流；
- 依赖倒置（域依赖 core，core 不依赖域）；
- 配置与代码分离；
- 双路径降级。

这些没有一个是 AI 引导出来的。它们是设计者先想清楚，再交给 AI 实现的。

## 执行失败在哪里

实现的失败不是笼统的「水平不高」，而是同一个具体失败在九处重复：把所有「理解」任务降级为「字符串包含」。

| **位置** | **意图** | **AI 的实现** |
|:--|:--|:--|
| `route` | 三问语义路由 | `"要" in text` |
| `grade` | 认识论判定 | 三个计数的 if |
| `cluster` | 本质相同合并 | 词集合重合度 ≥ 0.5 |
| `expand_observation` | 从情绪提取外部观察 | `sample.to_string()` |
| `to_fragment` | 提炼母题 | `source_title.clone()` |
| `extract_packaging` | 从正文提取包装文案 | 首句 / 5-15 字句 / 尾句 |
| `has_concrete_detail` | 判断含具体细节 | 命中 13 个单字动词 |
| `is_same_essence` | 判断本质相同 | 分词重合度 |
| `is_refuted` | 判断被推翻 | `"不是"` 子串 |

九个位置，九个同一个失败。这不是粗心——粗心会随机出错。这是一个系统性的降级：只要遇到「需要理解」的地方，就退化成「字符串包含」。

为什么会这样？因为 AI 不知道「理解」是可以委托给 LLM 的。它看到 `route(text: &str) -> Destination`，就以为这是一个纯字符串问题，没有意识到（或没有被明确告知）这是一个语义问题，应该调用 `Engine::judge`。

core 里明明有 `Engine::judge`。一个成熟的实现会在 `route` 里看到「这需要理解文本」，然后想到「core 里有 judge」，进而调用它。AI 没看到那层关联。

这就是「AI 实现水平不高」的具体形态：不是写不出正确的代码，而是看不到设计者留下的接口。设计者留下了 `LlmExtractor` 和 `Engine::judge`，AI 用它们旁边的空位写下了 `"要" in text`。

## 这份代码说明了什么

这是一份由懂业务的人设计的架构，被一个不懂业务的执行者填上了内容。

- 架构是深思熟虑的——类型、状态机、接口、doc，都不是随便写的；
- 内容是机械降级的——所有需要理解的地方都退化成字符串匹配；
- 二者不兼容——因为架构要求「理解」，实现提供「匹配」。

这解释了「形状精密，内容为空」。它不是设计者的失误，也不是架构的缺陷，是执行没有跟上设计。最令人遗憾的不是 AI 没写好某个函数，而是 AI 没看到设计者在 `Engine::judge` 那里留下的接口，于是从零开始写了一套启发式，而这套启发式的每一个判据都比 `Engine::judge` 能给出的弱。

## 把执行接上设计

这不是「修 bug」，而是把九处降级换成对 core 接口的调用：

- `route(text)` → 调用 `Engine::judge(route_rules, [text])`，返回 `Destination`；
- `grade(...)` → 调用 `Engine::judge(grade_rules, ...)`，返回 `InsightGrade`；
- `cluster(items)` → 调用 `Engine::judge(cluster_rules, items)`；
- `expand_observation(sample)` → 调用 `LlmExtractor::extract(...)`；
- `to_fragment(...)` → 调用 `LlmExtractor`；
- `extract_packaging(...)` → 调用 `LlmExtractor`。

AI 已经写好了一个让这些降级可以被替换的 API——`LlmExtractor` 和 `Engine::judge`。只要把每个降级函数的函数体改成调用它，设计意图就恢复了。

`RuleBasedExtractor` 的存在说明了设计者的预期：降级版本应该作为 fallback，而不是主实现。现在它变成了主实现。

## 分类体系没有落地

问题的根比「AI 把理解降级成字符串匹配」更深：设计者的三分法没有落到代码里。

| **是什么** | **谁定** | **例子** |
|:--|:--|:--|
| type | 系统预置 | 代码 `BlockType::{Heading, Bullet, ...}` |
| category | 业务类别 | 业务 `Insight` / `Profile` / `Roadmap` / ... |
| tag | KV 标签 | 数据 `source: "2024-03-15"` |

按这个框架看，`Destination` 是 category，不是 type。它由业务定义，会随业务变——今天是五个去向，明天可能加一个，不该是编译期固定的 enum。但代码把它写成了 type：

```rust
pub enum Destination {
    Insight, Profile, Roadmap, Intention, Journal,
}
```

这是 type 的做法——编译期穷举、不可扩展、改了要重编。

当 category 用 type 来实现，会发生三件事：

1. 它逼你写死判据。`route` 必须把「哪些话属于 Insight」翻译成代码，所以才有 `COGNITION_KEYWORDS` 这类东西。如果它是 category，判据应该来自配置（Artifact YAML），而不是硬编码。
2. 它让 LLM 用不上。category 本该是数据——LLM 读规则、给分类；type 是编译期概念，LLM 进不来。
3. 它让 AI 只能退化。AI 看到 enum，看到 `fn route(text) -> Destination`，没有别的选择，只能写 `if text.contains(...)`——因为 enum 就是「你要用代码判」的信号。

这不是 AI 笨，是类型选错了，把 AI 引导到了错误的方向。

如果当初写成 category，系统会长成这样（示意）：

```rust
pub struct Category {
    pub name: String,           // "insight"
    pub description: String,    // "想通了什么"
    pub rules: String,          // 判据，自然语言
}

fn route(text: &str, categories: &[Category]) -> Category
```

判据从 Artifact YAML 来，由 LLM（`Engine::judge`）执行。AI 看到这个签名就没有硬编码的余地——它必须去读 categories，必须调用判据，于是被引导对了。

链条是这样的：

- `enum Destination` 隐含「判据是代码」；
- 代码判据必须确定；
- 确定判据只能是字符串匹配；
- 字符串匹配必然捕捉不到语义。

不是 AI 选择降级，是类型选择让 AI 只能降级。一句话：`Destination` 是 category，代码把它做成了 type，这个错位是后面一切降级的源头。

## classify 与 route

`classify` 就是 `texts.map(route)`。它没有引入任何新逻辑，只是批量包装。

AI 会加这一层，是编程惯性。很多语言和框架的习惯是「底层函数 + 批量包装」，比如 `Iterator::next` + `collect`、单条 `read` + 批量 `read_all`。这是从性能优化场景带出来的——批量可以做并行、做缓冲。但 `classify` 什么都没做，它就是循环。在语义分类场景里，这个分层没有对应的现实：分一句话和分一批话是同一件事，只是数量不同。

这个分层有两个代价：

- 模糊了意图：分层让人以为 `classify` 是主 API、`route` 是细节，于是 `route` 的降级看起来像内部实现，不再被追问对不对；
- 掩盖了真正的问题：如果只有一个 `classify(text) -> Category`，它的判据是语义还是字符，一眼可见；分成两层后，问题被藏在「内部函数」里。

分层不是中性的，它给了问题一个藏身之处。合并后应该只有一个：

```rust
fn classify(text: &str) -> Category
```

批量由调用方自己 map。真需要批量时，可以加一个薄包装，但不要把它当作第二层概念——因为它本来就不是第二层，而是同一个概念的两种用法。

在只有 category、没有 type 的分类体系里，`classify` 的语义很清楚：给一段文本定一个 category。它不该有两层，因为 category 只有一个来源。分成 `route` + `classify`，是把「单条 vs 批量」这个无关的数量区别，伪装成了「判断 vs 分类」这个有关的语义区别。

## 分类是一次判断还是三件事

设计者的 `classify` 不是「给一个标签」，而是「一次把这段文本认识清楚」——它属于什么类型、归哪个类别、有什么属性。三个维度同时输出，一次判断。

但代码把它拆成了三件事：

```rust
route(text) -> Destination        // 只回答 category
grade(text, ...) -> InsightGrade  // 只回答「分级」
// tag 相关根本没有分类入口
```

不是三个维度一起判，而是三个独立函数各判一个维度。AI 看不到「这是同一个 `classify` 的三个面向」，只看到三个函数、三个返回类型，以为它们是三件事，于是每个都用关键词匹配顶上。

如果签名是一次回答三个维度：

```rust
fn classify(text) -> { type, category, tags }
```

AI 就没有退路。它不能只对某一个维度写 `if contains`，因为必须一次回答三个问题；而一次回答三个问题，关键词做不到。这时它必须意识到「这是语义判断」，也就必须去找 LLM。拆开之后，每个维度都小到看起来「用关键词能做」。

这不是命名分歧，而是「分类是一件事还是三件事」的分歧：

- 设计者的理解：分类是一次认知判断，三个维度是它的输出；
- 代码的实现：分类是 N 个独立判断，每个函数负责一个。

前者要求 LLM，后者允许关键词。AI 选了后者，因为它看到的是后者。

至于「为什么叫 `route` 不叫 `classify`」——设计者心里的 `classify` 是三合一的，而代码里的 `route` 只是其中一维。问题不是命名错了，是三维的 `classify` 被拆成了一维的 `route`。拆完之后，AI 就失去了「必须理解语义」的压力。

设计者的理解更正确，因为其中有一个约束，而 AI 的理解里没有：「三个维度来自同一次判断」。这意味着：

- 不可能只判对其中一个；
- 不可能用三种不同的方法判同一个东西；
- 必须真的理解这段文本，才能同时回答三个问题。

这个约束是对的，因为现实里 type、category、tag 本来就是一次认知的三个切面。人看到一个东西，同时知道「它是什么类」「它归哪儿」「它有什么属性」，这不是三次判断，是一次判断的三个输出。

相反，「分类 = `route` + `grade` + `tag` 三个独立函数」丢掉了这个约束。三个函数各自独立：`route` 可以只用关键词，`grade` 可以只用计数，`tag` 可以直接抄字段，没有任何机制要求它们来自同一次判断。于是每一个都可以局部退化，用各自最便宜的启发式顶上。结果三个都是错的，而且互不相关。

「同一次判断」这个约束之所以关键，是因为有它时 AI 没有退路：一个函数要同时回答三个问题，关键词做不到，它必须找到能同时处理三个维度的东西，而这样的东西只有一个——理解。于是它必须接入 LLM。去掉约束后，每个维度都小到看起来「用关键词能做」，AI 便逐维退化。约束不是为了好看，是为了防止退化。

不过有一处需要修正：type 和 category 确实都是语义判断，但 tag 不完全是。tag 是 KV，很多 tag 来自元数据而不是文本语义——日期从文件名来，来源从路径来，版本从上下文来。这些不是「理解」出来的，是「提取」出来的。所以更准确的说法是：type 和 category 是一次判断（语义），tag 是另一次提取（元数据）。两者同属分类场景，但判据不同。如果把它们强行捆在一起，AI 可能为了「统一」而把 tag 也硬做成语义判断，反而错。

所以设计者对分类的理解更正确，因为它保留了「一次判断输出多个维度」这个约束。这个约束是整个设计里防止 AI 退化的最后一道防线，不应该被拆开。

## 重构从意图的可判定化开始

前面所有缺陷，不是代码写错了，而是意图从来没有被写成「可判定的形式」。

看 `route` 的 doc：「想通了什么」→ insight、「我是什么样的人」→ profile……这句话是意图的描述，不是判据。它告诉你「应该有哪几类」，但没告诉你「每一类的边界在哪」。AI 拿到这句话只能自己猜，猜出来的就是关键词表。关键词表不是 AI 偷懒，是「意图不够可判定」的产物。

所以重构的第一步是写判据。对每个核心概念写三样东西：

1. 它是什么（意图）；
2. 它怎么被判断（判据）；
3. 边界在哪（正例、反例、易混例）。

拿 `Insight` 举例：

- 意图：这条文本在说「我发现了什么」；
- 判据：不是「有没有『发现』这个词」，而是它是不是在表达一个新认识；
- 边界：正例是「我发现我不适合做产品」「原来问题在这里」；反例是「我要放弃」（这是意愿，不是认识）、「这个方法很重要」（这是评价，不是新认识）；易混例是「我一直在想……」（可能是认识，也可能是过程）。

注意易混例——「可能是」的地方正是 AI 会降级的地方，必须给出裁决。这才是判据。有了它，AI 才可能实现对。

判据写完，类型会自动变。不要先改类型，类型是判据的产物。判据写完你会发现：

- `Destination` 不该是 enum，而是一组数据驱动的类别（有 name、description、判据）；
- `route` 签名变成 `fn(text, categories) -> Category`，判据从 categories 来，不能硬编码；
- `route` 里再也写不出 `text.contains("要")`，因为「要」不构成判据。

类型的形状会阻止降级。不需要「告诉 AI 要理解语义」，只需要「让类型的形状没有别的地方可走」。

从哪开始？不是从最抽象的原则，也不是从最容易的地方，而是从最明显的坏点——`route`：

- 它坏得最明显（「要」的问题）；
- 它影响最大（入口）；
- 它牵动最多——修它会连锁触发：修 `route` → 发现 `Destination` 类型错位 → 发现 category 该数据驱动 → 发现 `Artifact` 该定义 category → 发现 `rules.rs` 该被真正使用 → 发现 `LlmExtractor` 该被接上。

一个坏点牵出一串结构问题。这就是「从最明显的坏点开始」的价值——不是因为它最重要，而是因为它牵动最多。

顺序是：

1. 不改代码，为 `Destination` 的五个类别写判据，含正例反例；
2. 改类型，把 `Destination` 从 enum 改成 category 类型；
3. 改签名，`route(text, categories) -> Category`；
4. 接 LLM，判据交给 `Engine::judge`；
5. 重复，把这套模式应用到 `grade`、`cluster`、`expand_observation`……

每一步只动一个东西，不要一次重写整个项目。

如果只做一件事，就写 `Destination` 五个类别的判据——不写代码，不改文件。写完，下一步会自动显形，因为判据和当前类型的错位会自己暴露出来。

一句话：重构不是从代码开始，是从意图开始。把意图写成可判定的判据，类型会自己跟着变，AI 就不会再降级。起点是 `route` 的五个类别，各写一段判据。

## Destination 不该存在

`Destination` 不该存在，它是 `Artifact` 的 key。

```rust
// Destination 说「去哪」
enum Destination { Insight, Profile, Roadmap, Intention, Journal }

// Artifact 说「到了之后怎么读」
struct Artifact { document_type, title, description, sections }
```

「去哪」和「怎么读」本该是同一件事：`Destination::Insight` 就是「送去 insight 文档」，而 insight 文档的读法，是一份 `insight.yaml` 加载出来的 `Artifact`。`Destination::Insight` 本质上就是 `insight.yaml` 的名字。一个 enum 变体对应一个 YAML 文件，这就是 key 和 value 的关系。

所以正确的结构是：

```rust
struct Artifact {
    name: String,          // "insight" ← 这就是 Destination
    document_type: String,
    title: TitleRule,
    description: DescriptionRule,
    sections: SectionsRule,
    // ...
}
```

`Destination` 不在了，它的五个变体变成五份 `Artifact` 的 `name` 字段。`route` 变成：

```rust
fn route(text: &str, artifacts: &[Artifact]) -> &Artifact
```

输出不再是枚举变体，而是一份具体的 `Artifact`。

这解决了前面所有问题：

1. category 被写成 type——现在改成 `Artifact.name`，数据驱动，可增可删；
2. 判据硬编码——现在判据来自每份 `Artifact`，`insight.yaml` 里写「什么时候算 insight」；
3. `Artifact` 悬空——它加载了却没人用，现在它是 `route` 的输入，天然被使用；
4. `LlmExtractor` 悬空——它需要 `Artifact` 作参数，现在 `route` 也返回 `Artifact`，整条流水线终于有了共同的数据类型。

`Journal` 是特殊情况：它不在这个映射里——它不是「送去某处」，而是「留在原地」。它不是 `Artifact`，是默认值：

```rust
fn route(text, artifacts) -> Option<&Artifact>
// None = 没有匹配 = 留在 journal
```

真正的 `Destination` 只有四个：Insight、Profile、Roadmap、Intention，`Journal` 是它们的补集。

一句话：`Destination` 是 `Artifact` 的名字被硬编码成了 enum。拆掉这个硬编码，`route`、`Artifact`、`LlmExtractor`、`rules.rs` 全部自动接上。这是整个重构的第一个杠杆。

## 同类问题的完整清单

「类似」指的是：本该是 category 的东西被做成了 type，或者反过来。按三分法梳理，共有五个方向。

### category 被做成了 type

与 `Destination` 是同一个错位。

| **东西** | **它本该是** | **被做成了** |
|:--|:--|:--|
| `Destination` | `Artifact` 的 key | enum |
| `InsightGrade` | insight 的分级类别 | enum |
| `MergeAction` | 合并策略类别 | enum |
| `Verb`（scan / judge / merge） | workflow 定义的动词 | `Step.verb: String` |

共同后果是：判据没地方来，只能硬编码；加一类要改代码、重编译；LLM 接不进来；AI 只能用字符串匹配顶。`InsightGrade` 尤其明显，它被写在 `build_section` 里：

```rust
if raw.title.contains("已确认") { Confirmed }
else if raw.title.contains("假说") { Hypothesis }
```

「已确认」「假说」是 category 的名字，硬编码在代码里。

### type 被做成了 String

反方向——本该固定取值的东西，被写成任意字符串。

| **东西** | **它本该是** | **被做成了** |
|:--|:--|:--|
| `split_by` | H1..H6 | String |
| `title.source` | 几个固定来源 | String |
| `description.location` | 几个固定位置 | String |
| `Step.verb` | Scan / Judge / Merge | String |

共同后果是：编译期不校验，`split_by: "h7"` 也能加载；运行期才可能报错（`UnknownVerb`）；或者根本不报错（`split_by` 没人读）。

### category 被做成了计数

`should_promote_to_profile` 的错位：

```rust
fn should_promote_to_profile(item, application_count: usize) -> bool {
    application_count >= 3
}
```

意图是「命题反复套用、稳定为思维框架」，其中「稳定为思维框架」是 category 判断——这个命题的性质是什么。但签名给了 `usize`，把 category 问题伪装成了计数问题。同一错位还有 `grade` 的 `occurrence_count >= 2`。

### category 被做成了关键词表

最隐蔽的一种：

```rust
const INTENT_KEYWORDS: [&str; 12] = ["要", "不要", ...];
```

关键词表是 category 的降级形式——用一组词近似一个语义类别。它看起来像「分类配置」，实际是硬编码判据。同一错位还有 `has_concrete_detail` 里的 13 个动词。

### 主次关系被抹平

`SemanticExtractor` trait：

```rust
impl SemanticExtractor for LlmExtractor { ... }
impl SemanticExtractor for RuleBasedExtractor { ... }
```

意图是 LLM 首选、规则降级，但 trait 让它们平级，主次关系消失。同一错位还有 `Option<String>`——它让「缺失」看起来正常，于是 `reason: None` 永远不填。

根源是三分法没有落到代码里。设计者心里有 type、category、tag，代码里所有东西却都变成了 enum 或 String：该是 category 的成了 enum 或 String，该是 type 的成了 String，该是 tag 的没有对应结构。三个概念在代码里只活下来一个半。

一句话：同一个错位，五个方向——

1. category → type（`Destination` / `InsightGrade` / `MergeAction`）；
2. type → String（`split_by` / `verb`）；
3. category → 计数（`should_promote`）；
4. category → 关键词表（`INTENT_KEYWORDS`）；
5. 主次 → 平级（`SemanticExtractor` / `Option`）。

全部是「三分类没落地」这同一个病的不同症状。

## 结语

这份代码整体上不是半成品，而是一份完成的设计加上一份未完成的填充。

- 设计者想清楚了：创始人工具是什么、认知如何分类、创作如何提炼、规则如何外部化、LLM 扮演什么角色；
- 执行者把这些填成了：字符串包含、恒等返回、占位分支。

设计者把意图写在了代码形状里，而不是代码体里——形状是完整的、自洽的、有理论的；实现是降级的、一致的、机械的。所以能从代码里捕捉到的业务意图和设计意图，几乎全部。

用一句话总结：一个懂业务和技术的资深专家写了一份「产品在做什么」的完整规格；执行者把这份规格翻译成 Rust 时，在每一个「这里需要理解语义」的地方，都填上了字符串匹配。而修复它的方式不是重写，是认出那些地方，把它们接到设计者早就准备好的接口上。
