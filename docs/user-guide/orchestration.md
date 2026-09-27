# 编排与状态流

解析产出的模型是计算的输入。这一组编排把「扫描、判断、合并」按序组装：Dart 侧是 `MemoryBloc` 与 `FictionBloc` 的方法调用，Rust 侧是两个 statig 状态机加一批纯函数，行为逐条对齐。

## memory 四步流程

扫描 → 过滤 → 聚类 → 合并，每步只做一件事：

1. 扫描：按关键词表定位候选，认知类（发现、原来、其实…）、意图类（要、别、澄清…）、方向类（决定、优先、待定…）、特征类（我总是、我害怕…）
2. 过滤：三问路由定去向，意图优先于方向与认知，拿不准留 journal
3. 聚类：同一认识跨日期合并，判断依据是本质相同而非措辞相似
4. 合并：新线索增条目、已有就地改写、被推翻替换、重复留互链

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~dart
final bloc = MemoryBloc(engine: Engine(llmClient: client));
final classified = bloc.processJournal(workflow, repo.sets.first.journals);
final destination = bloc.route(text); // 三问路由
final grade = bloc.grade(text, occurrenceCount: 3); // 证据分级
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~rust
// 状态机：事件进、状态出，走到哪一步一目了然
let mut machine = MemoryFlow.state_machine();
machine.handle(&MemoryEvent::Scan(entries));
machine.handle(&MemoryEvent::Route);
machine.handle(&MemoryEvent::Cluster);
let classified = machine.state().classified();

// 判断规则是纯函数
let destination = route(text); // 三问路由
let grade = grade(text, 3, false, false); // 证据分级
~~~

:::
```

Rust 侧事件走错步不理会：还没扫描就 `Route`、还没路由就 `Cluster`，状态原地不动。

## fiction 三步提炼

取样 → 观察展开 → 落实片段。中间必须有观察展开：直接从情绪跳到场景素材会产出「金句」而不是「画面」，先提具体场景、动作、细节，再长出片段。

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~dart
final bloc = FictionBloc(engine: Engine(llmClient: client));
final fragment = bloc.extract(diary);
print('${fragment.motif} / ${fragment.scene}');
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~rust
let mut machine = FictionFlow.state_machine();
machine.handle(&FictionEvent::Sample(diary.clone()));
machine.handle(&FictionEvent::Expand);
machine.handle(&FictionEvent::Settle);
let fragment = machine.state().fragment();

// 一步到位的便捷入口
let fragment = FictionFlow::extract(&diary);
~~~

:::
```

场景素材是 50 字钩子，母题卡片是一句话说清写什么。

## 章节编号轴

编号轴跨阶段共用，三条规则各有一个判断：

- `assignNumber`：优先填补预留空号（宁空勿移），否则当前最大号加一
- `findGaps`：返回 1 到最大号之间的空号，前言 `0_` 与未编号不占号
- `checkStage`：该阶段章节都已编号且无空位才算完成，否则列出缺号

同一序号在不同阶段的多份文件是同一章的不同版本。阶段流转方向由目录编号给出（`1_灵感 → 2_场景 → …`），阶段名各小说不同，不硬编码。

## 包装文案

作品进定稿阶段时从正文提取三样，各承担不同信息、不互相重复：标题取首句（核心矛盾与钩子），一句话简介取 5 到 15 字的句子（增量信息），立意取末句（价值维度）。

## 读终态还是订阅

一次跑完、驱动者自己看结果，读快照就够，四个示例（`parse_memory`、`parse_fiction`、`memory_workflow`、`fiction_workflow`）都是这一类：

~~~rust
let classified = machine.state().classified();
~~~

进程内有多个消费者要实时知情——进度显示、流水记账、闸门提示——才需要订阅：状态每次变化推给订阅者，驱动方对观察方一无所知，观察方可增可减。Rust 侧演示在 `examples/agent_subscribe.rs`：

~~~rust
// 每次转移后，快照推给所有订阅者（statig 的 after_transition 钩子）
fn broadcast(&mut self, _from: &TaskState, to: &TaskState, _ctx: &mut ()) {
    for sender in &self.subscribers {
        let _ = sender.send(to.clone());
    }
}
~~~

订阅是进程内多个消费者的应用形态，不进工具箱的领域流水线；工具箱只保证 `machine.state()` 与挂钩点可用。选型与取舍的完整论证见[状态机设计](../dev-guide/state-machine.md)。
