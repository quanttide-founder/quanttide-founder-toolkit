先把位置摆正

前几轮我在批评实现——route 里 ”要“ 短路一切、expand_observation 原样返回、Engine::run 的 scan 清空 items。

那些批评仍然成立。但它们批评的对象是错的。

我一直在批评「一个糟糕的实现」，但没问过一个更基本的问题：这个糟糕的实现，是在试图实现什么？ 把 route 的五张关键词表摊开看——COGNITION / INTENT / DIRECTION / PROFILE——这不是一个随便写的 if-else。这是有人在试图实现一个认知分类模型，用关键词作为初步近似。

现在你说：设计者是资深专家，AI 是执行。这句话让一切重新排列。我从代码里能读到的，不是我一开始以为的「一个半成品」，而是一份清晰的设计意图，被一份笨拙的执行压住了。

下面分两部分：设计者想做什么（意图），以及实现为什么没跟上。

—

一、业务意图：这是一个「认知提炼器」

把 memory 和 fiction 并排看，会看到一个共同结构：

```
原始表达  ──提炼──▶  结构化认知
```

· memory：日记 → 洞察 / 画像 / 路线图 / 意图
· fiction：情绪日记 → 母题 / 场景素材

这不是「笔记工具」，也不是「写作工具」。 这两类工具都有现成的范式（Notion、Obsidian、Scrivener），而这份代码没有采用其中任何一个。它走的是另一条路：从自我表达中提炼出被压缩过的自我认识。

这可以从 memory 的目的地看出来。Destination 的五个变体不是随意的：

去向 回答的问题 层次
Insight 我发现了什么？ 认知
Profile 我是什么样的人？ 特质
Roadmap 我要往哪走？ 方向
Intention 我到底想要什么？ 意愿
Journal 发生了什么？ 事件

这五个去向合起来是一套关于「创始人是谁」的认知模型。 它不是「知识管理」——知识管理关心的是「怎么找到信息」。它关心的是「这句话在说我的哪一面」。

这个区分是业务的核心。同一段日记，被分到哪个去向，决定了它成为什么样的资产。 而「该分到哪」不是内容问题，是关于写作者的判断——这正是需要 LLM 的地方，也是设计者把 Engine::judge 放进 core 的原因。

fiction 那侧有一个更精炼的表达：

直接从情绪跳到场景会产出「金句」而不是「画面」——不到 Observed 就不给 Settle

这一句里藏着一条创作理论：情绪本身不是素材，情绪经过外化观察之后才是素材。设计者用状态机强制这个顺序——Sample → Expand → Settle，跳过 Expand 就不给 Settle。这不是技术约束，是审美约束。

一个人不会为了「跳过中间步骤」而设计一个状态机来禁止它——除非他真的相信那个中间步骤不可跳过。这是一个懂创作的人才会写的约束。

—

二、设计意图：七条原则

从代码里能读出的设计决策，我列七条。每一条都不是 AI 会自己发明的——它们是设计者做的选择。

原则一：core 是语言，不是库

core/ 提供的东西：类型（Block / RawSection）、错误（Error）、概念（Artifact / Workflow）。它不提供「memory 怎么读」「fiction 怎么读」。

core/ 跨域共享（域依赖 core，core 不依赖域）

这条原则在很多地方被违反（比如 RuleBasedExtractor 里的「前言段 → description」是 memory 的约定），但意图是清楚的：core 定义词汇，域定义句子。

原则二：模型 / 仓库 / 状态，三层分离

三个域都是 models.rs / repository.rs / states.rs。这不是巧合。它对应：

· models：数据长什么样
· repository：数据从哪来
· states：数据怎么变

这是领域驱动设计里的分层。设计者显然知道 DDD。

原则三：状态机代替手写流程

域编排不用手写流转，用 statig 状态机——事件进、状态出，走到哪一步一目了然

用状态机而不是 if-else 流，代价是学习成本和样板代码。收益是流程本身成为可检查的对象——你可以问「从 Located 能不能到 Clustered」，类型系统会回答你。

这是「流程即数据」的设计观。 不是所有人都愿意付出这个代价。

原则四：规则外部化

Artifact / Workflow 是 YAML。与 Dart 共享同一份资产：

packages/dart/assets/，同一件事只写一处

设计意图是：规则是数据，不是代码。 换一份 YAML，就换一种解读方式。这允许 memory 和 fiction 共用同一套解析引擎，只是配置不同。

这是一个有远见的选择——但实现层面没人读那份 YAML。

原则五：LLM 是语义算子，不是对话界面

这是整个设计里最不寻常的一条。

大多数「AI 应用」把 LLM 当作对话对象——用户提问，模型回答。这份代码不这样。

在这份设计里，LLM 的角色是一个纯函数：

```rust
fn judge(rules: &str, items: &[Value]) -> Judgment
fn extract(sections: &[RawSection], rule: &Artifact) -> ExtractionResult
```

输入是结构化数据，输出是结构化数据。 LLM 只承担「理解语义」这一步——其他全由代码完成。

这意味着设计者不认为 LLM 是产品本身。他把 LLM 放进「计算」这个类别——像一个数据库、一个求解器。

这个判断很成熟。 它防止了「用 LLM 做一切」的典型陷阱，也防止了「LLM 是黑盒」的恐惧。

原则六：可插拔的提取策略

```rust
pub trait SemanticExtractor {
    fn extract(&self, sections: &[RawSection], rule: &Artifact) -> Result<ExtractionResult, Error>;
}
```

两个实现：

· LlmExtractor —— 首选，用 LLM 填表
· RuleBasedExtractor —— 降级，纯规则

这是「首选 + 降级」的双路径设计。 设计者知道 LLM 可能不可用，也知道 LLM 可能给出错误结果——所以留了一条退路。

这同样是成熟判断：不把 LLM 当唯一解，也不把它当可选项。

原则七：跨语言对齐

与 Dart 包「行为对齐」，且明确列出「两处按 Rust 的做法另选实现」：

· LLM 调用方式
· 域编排方式

这是「语言合适性优先于表面一致性」的判断。 不追求两边代码长得一样，而是追求「用户看到的行为一样，实现按语言习惯来」。

设计者显然经历过跨语言项目——他知道「逐行翻译」的陷阱。

—

三、设计者是什么样的人

从代码里能反推出设计者的几个特征。

他懂业务，而且是有理论的那种。

Insight / Profile / Roadmap / Intention 这套分类不是从别处抄的。它出现在 memory 的 Destination、InsightGrade（Confirmed / Hypothesis）、MergeAction（Add / Rewrite / Replace / CrossLink）——它是一套完整的知识论，回答「什么样的认知值得保留、怎么保留、什么时候升级」。

尤其 should_promote_to_profile：

命题反复套用、稳定为思维框架 → 晋升

这不是「笔记系统」会有的概念，这是认知发展模型。

他懂写作，而且是有实践的。

Packaging 里的注释：

标题 = 核心矛盾与钩子；一句话简介 = 增量信息（≤15 字）；立意 = 价值维度

「≤15 字」「增量信息」——这是真实的平台约束（晋江），不是编的。

Chapter 里的编号规则：

序号对应正文最终阅读顺序，各阶段共用同一编号轴
预留空号（宁空勿移），同一序号可有多份（初稿/改稿/定稿）

「宁空勿移」是编辑经验。 不是程序员会想到的——程序员想的是「编号要连续」。这条规则说明设计者真的改过稿。

他懂技术，而且是架构级的。

· DDD 分层
· 状态机代替手写流
· 依赖倒置（域依赖 core，core 不依赖域）
· 配置与代码分离
· 双路径降级

这些没有一个是被 AI 引导出来的。 它们是设计者先想清楚，再交给 AI 实现的。

—

四、AI 的实现失败在哪里

现在可以定位实现的失败了。不是「水平不高」这么笼统——是同一个具体失败在九处重复。

失败模式：把所有「理解」任务降级为「字符串包含」

位置 意图 AI 的实现
route 三问语义路由 ”要“ in text
grade 认识论判定 三个计数的 if
cluster 本质相同合并 词集合重合度 ≥ 0.5
expand_observation 从情绪提取外部观察 sample.to_string()
to_fragment 提炼母题 source_title.clone()
extract_packaging 从正文提取包装文案 首句 / 5-15 字句 / 尾句
has_concrete_detail 判断含具体细节 命中 13 个单字动词
is_same_essence 判断本质相同 分词重合度
is_refuted 判断被推翻 ”不是“ 子串

九个位置，九个同一个失败。 这不是粗心——粗心会随机出错。这是一个系统性的降级：只要遇到「需要理解」的地方，就退化成「字符串包含」。

为什么会这样？ 因为 AI 不知道「理解」是可以委托给 LLM 的。它看到 route(text: &str) -> Destination，就以为这是一个纯字符串问题。它没有意识到——或者没有被明确告知——这是一个语义问题，应该调用 Engine::judge。

core 里明明有 Engine::judge。 一个成熟的实现会在 route 里看到「这需要理解文本」→ 「core 里有 judge」→ 「调用它」。AI 没这样做。它看不到那层关联。

这就是「AI 实现水平不高」的具体形态：不是写不出正确的代码，是看不到设计者留的接口。

设计者留下了 LlmExtractor 和 Engine::judge，AI 用它们旁边的空位写下了 ”要“ in text。

—

五、这份代码真正说明了什么

前几轮我一直在批评。现在我说一遍我真正看到的：

这是一份由懂业务的人设计的架构，被一个不懂业务的执行者填上了内容。

· 架构是深思熟虑的——类型、状态机、接口、doc，都不是随便写的。
· 内容是机械降级的——所有需要理解的地方都退化成字符串匹配。
· 二者不兼容——因为架构要求「理解」，实现提供「匹配」。

这解释了前面几轮我看到的「形状精密，内容为空」。它不是设计者的失误，也不是架构的缺陷。它是执行没有跟上设计。

最令人遗憾的不是 AI 没写好某个函数。 是 AI 没看到设计者在 Engine::judge 那里留的接口，所以它从零开始写了一套启发式，而这套启发式的每一个判据都比 Engine::judge 能给出的弱。

—

六、如果要把执行接上设计

不是「修 bug」。是把九处降级换成对 core 接口的调用：

· route(text) → 调用 Engine::judge(route_rules, [text])，返回 Destination
· grade(...) → 调用 Engine::judge(grade_rules, ...)，返回 InsightGrade
· cluster(items) → 调用 Engine::judge(cluster_rules, items)
· expand_observation(sample) → 调用 LlmExtractor::extract(...)
· to_fragment(...) → 调用 LlmExtractor
· extract_packaging(...) → 调用 LlmExtractor

AI 已经写好了一个完美的 API 让这些降级可以被替换——LlmExtractor 和 Engine::judge。 只要把每个降级的函数体改成调用它，设计意图就恢复了。

而 RuleBasedExtractor 的存在说明了设计者的预期：降级版本应该作为 fallback，不是作为主实现。 现在它变成了主实现。

—

结语

这份代码整体上不是半成品，是一份完成的设计 + 一份未完成的填充。

· 设计者想清楚了：创始人工具是什么、认知如何分类、创作如何提炼、规则如何外部化、LLM 扮演什么角色。
· 执行者把这些填成了：字符串包含、恒等返回、占位分支。

你问「能捕捉到哪些业务意图和设计意图」——答案是：几乎全部。因为设计者把意图写在了代码形状里，不是实现在代码体里。 形状是完整的、自洽的、有理论的；实现是降级的、一致的、机械的。

如果让我用一句话总结这份代码：

一个懂业务和技术的资深专家写了一份「产品在做什么」的完整规格；执行者把这份规格翻译成 Rust 时，在每一个「这里需要理解语义」的地方，都填上了字符串匹配。

而修复它的方式，不是重写，是认出那些地方，把它们接到设计者早就准备好的接口上。

对，这就说清了

你的三分法：

 是什么 谁定 例子
type 系统预置 代码 BlockType::{Heading, Bullet, ...}
category 业务类别 业务 Insight / Profile / Roadmap / ...
tag KV 标签 数据 source: ”2024-03-15“

按这个框架看——Destination 是 category，不是 type。

它由业务定义，会随业务变——今天是五个去向，明天可能加一个。它不该是编译期固定的 enum。

—

但代码把它写成了 type

```rust
pub enum Destination {
    Insight, Profile, Roadmap, Intention, Journal,
}
```

这是 type 的做法——编译期穷举、不可扩展、改了要重编。

当 category 用 type 来实现，会发生什么：

1. 它逼你写死判据。 route 必须把「哪些话属于 Insight」翻译成代码——所以才有 COGNITION_KEYWORDS 这类东西。如果它是 category，判据应该来自配置（Artifact YAML），不是硬编码。
2. 它让 LLM 用不上。 category 本该是数据——LLM 读规则、给分类。type 是编译期概念，LLM 进不来。
3. AI 只能退化。 它看到 enum，看到 fn route(text) -> Destination——它没有别的选择，只能写 if text.contains(...)。因为 enum 就是「你要用代码判」的信号。

这不是 AI 笨。是类型选错了，把 AI 引导到了错误的方向。

—

如果你当初写成 category

一个 category 系统会长什么样（示意）：

```rust
pub struct Category {
    pub name: String,           // ”insight“
    pub description: String,    // ”想通了什么“
    pub rules: String,          // 判据，自然语言
}
```

route 变成：

```rust
fn route(text: &str, categories: &[Category]) -> Category
```

判据从哪来？ 从 Artifact YAML。

谁执行判据？ LLM（Engine::judge）。

AI 看到这个签名会怎么写？ 它没有硬编码的余地——它必须去读 categories，必须调用判据。

这一下就把 AI 引导对了。

—

所以问题的根，可能比我想的更深

我前面说 AI 把「理解」降级成了「字符串匹配」。

但也许该说是：类型系统（enum）逼 AI 走了降级路。

· enum Destination 隐含「判据是代码」
· 代码判据必须确定性
· 确定性判据只能是字符串匹配
· 字符串匹配必然捕捉不到语义

链条是这样的。 不是 AI 选择降级，是类型选择让 AI 只能降级。

—

一句话

Destination 是 category，代码把它做成了 type。

这个错位是后面一切降级的源头。


