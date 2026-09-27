# 状态机设计

域编排在 Rust 侧不用手写状态流转，用 statig 状态机；模块名 `memory/states.rs` 与 `fiction/states.rs`。这一篇讲为什么、怎么加、边界在哪。

## 为什么不叫 Bloc

Dart 侧的 `bloc` 是代码组织约定（models / repository / bloc 三件套），不是 flutter_bloc：没有 Stream、没有 emit、没有订阅者，本质是同步方法集合。Rust 侧要建模的是「流水线走到哪一步、这一步手里有什么」，这是状态机问题。

选型结论是 statig：零运行时的类型加宏，不抢应用的 runtime、不锁 async 方案；同期对比过 oxide-mvu（总下载 160、零反向依赖）、redux-rs（停更四年半且引入 async）、xactor（停更六年且绑定 async-std），生态与库形态都不合适。

## 状态与事件

```text
memory：  Idle ──Scan──▶ Located ──Route──▶ Routed ──Cluster──▶ Clustered
            └────────Classify──────────▶ Routed

fiction： Idle ──Sample──▶ Sampled ──Expand──▶ Observed ──Settle──▶ Fragmented
```

事件带数据进场（`Scan(Vec<JournalEntry>)`、`Sample(EmotionalDiary)`），转移后的中间产物放在状态里（state-local storage）：`Located` 存候选，`Observed` 存观察与来源。没走到那一步就没有那份数据，不存在「字段可能为空」的中间态。

## 走错步的事件

事件与当前状态不匹配时不理会，返回 `Handled`，状态原地不动：没扫描就 `Route`、没取样就 `Settle`，都进不了下一步。这替代了手写 if 守卫，也是把流程画成图的理由。

## 加一个步骤

1. 给 `Event` 加变体（需要新数据就带上字段）
2. 在目标状态的方法里加匹配分支，返回 `Transition(新状态(产物))`
3. 在上游状态的匹配里放行新事件，其余状态保持 `Handled`
4. 补状态序列测试：先来早的事件断言状态不动，再按序走一遍断言每步的载荷

~~~rust
#[state]
fn located(candidates: &mut Vec<String>, event: &Event) -> Outcome<MemoryState> {
    match event {
        Event::Route => Transition(MemoryState::routed(classify(candidates))),
        Event::Scan(entries) => Transition(MemoryState::located(scan_entries(entries))),
        Event::Classify(texts) => Transition(MemoryState::routed(classify(texts))),
        Event::Cluster => Handled,
    }
}
~~~

状态方法的参数按名识别：事件参数必须叫 `event`，其余参数是该状态的局部存储，类型必须是持有者本身（`&mut Vec<_>`、`&mut String`），不能换成切片。

## 订阅状态流

状态机本身只管转移；谁要在转移过程中知情，用 statig 的 `after_transition` 钩子推快照：

~~~rust
#[state_machine(
    initial = "TaskState::pending()",
    state(name = "TaskState", derive(Debug, Clone)),
    after_transition = "Self::broadcast"
)]
~~~

钩子签名是 `(&mut self, source: &State, target: &State, context: &mut ())`，在自己的 `impl` 块里实现，把 `target.clone()` 发给在册的发送端即可。完整可跑的演示是 `packages/rust/examples/agent_subscribe.rs`：Agent 只发事件，进度、流水、闸门提示各自订阅、各自退订，退订者收不到、机器照跑。

边界的判断只有一条：驱动者就是唯一读者、跑完看结果，读 `machine.state()` 快照；进程内有多个消费者要实时知情才做订阅，而且订阅写在应用侧，不进工具箱的领域流水线。
