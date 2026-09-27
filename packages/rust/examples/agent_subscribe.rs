//! 「上层订阅状态流」演示：驱动方只发事件，观察方各自订阅。
//!
//! 场景取自 CLI 里跑工作流的 Agent（对照 qtcloud-work 的任务执行）：
//! 执行侧推进任务状态机，进度显示、流水记账、闸门提示各自订阅——
//! **驱动方不认识观察方，观察方可增可减**。
//!
//! ```sh
//! cargo run --example agent_subscribe
//! ```
//!
//! 两种读法的分工（对照 BLoC 的 `bloc.stream.listen`）：
//! - **取（pull）**：parse_memory / memory_workflow 那样，跑完 `machine.state()` 读快照——
//!   驱动者就是唯一读者，工具箱的领域流水线停在这一步
//! - **推（push）**：本例，每次转移由 `after_transition` 钩子把快照推给订阅者——
//!   进程内有多个消费者要实时知情时才需要，属应用形态，所以状态机在示例里定义

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use quanttide_founder::statig::prelude::*;

/// 状态机的事件：执行侧（Agent）只认事件，不认识任何观察方。
#[derive(Debug)]
enum Event {
    /// 开工，进入第一步。
    Start,
    /// 当前步骤收尾。
    StepFinished,
    /// 人工审批结果（闸门）。
    Approved(bool),
}

/// 任务流程（应用侧定义的机器——订阅是应用形态，不进工具箱）。
struct TaskFlow {
    /// 在册订阅者：每次转移，目标状态的快照推给每一位。
    subscribers: Vec<Sender<TaskState>>,
}

impl TaskFlow {
    fn new() -> Self {
        Self {
            subscribers: Vec::new(),
        }
    }

    /// 订阅状态流：拿到接收端，之后每次转移都会收到快照。
    ///
    /// 对照 BLoC：`bloc.stream.listen(...)`。
    fn subscribe(&mut self) -> Receiver<TaskState> {
        let (tx, rx) = mpsc::channel();
        self.subscribers.push(tx);
        rx
    }

    /// 每次转移后，把目标状态快照推给所有订阅者。
    fn broadcast(&mut self, _source: &TaskState, target: &TaskState, _context: &mut ()) {
        for sender in &self.subscribers {
            // 已退订者（receiver 丢弃）收不到，跳过即可——观察方增减不影响机器
            let _ = sender.send(target.clone());
        }
    }
}

#[state_machine(
    initial = "TaskState::pending()",
    state(name = "TaskState", derive(Debug, Clone)),
    after_transition = "Self::broadcast"
)]
impl TaskFlow {
    #[state]
    fn pending(event: &Event) -> Outcome<TaskState> {
        match event {
            Event::Start => Transition(TaskState::running("code-review".to_string())),
            _ => Handled,
        }
    }

    #[state]
    fn running(step: &mut String, event: &Event) -> Outcome<TaskState> {
        match event {
            // 第一步收尾 → 撞上人工闸门
            Event::StepFinished if step == "code-review" => {
                Transition(TaskState::awaiting_approval("release-audit".to_string()))
            }
            // 其余步骤收尾 → 完稿
            Event::StepFinished => Transition(TaskState::done()),
            _ => Handled,
        }
    }

    // 状态局部存储的持有者必须是 &mut String，不能换成切片
    #[allow(clippy::ptr_arg)]
    #[state]
    fn awaiting_approval(gate: &mut String, event: &Event) -> Outcome<TaskState> {
        match event {
            // 审批通过：闸门本身也是一步，接着执行它
            Event::Approved(true) => Transition(TaskState::running(gate.clone())),
            // 驳回：任务终止
            Event::Approved(false) => Transition(TaskState::done()),
            // 闸门未决，其余事件不理会
            _ => Handled,
        }
    }

    // 终态不收事件（无事件参数 = 谁来都不动）
    #[state]
    fn done() -> Outcome<TaskState> {
        Handled
    }
}

fn main() {
    let mut flow = TaskFlow::new();

    // ── 上层各自订阅（机器还没启动，驱动方对观察方一无所知）──
    let progress = flow.subscribe(); // ① 进度显示
    let ledger = flow.subscribe(); // ② 流水记账
    let gate_notice = flow.subscribe(); // ③ 闸门提示（会中途退订）

    let progress_handle = thread::spawn(move || {
        while let Ok(state) = progress.recv() {
            println!("  [进度] 状态 → {state:?}");
        }
    });
    let ledger_handle = thread::spawn(move || {
        let mut seq = 0;
        while let Ok(state) = ledger.recv() {
            seq += 1;
            println!("  [流水] 第 {seq} 笔：{state:?}");
        }
    });
    let gate_handle = thread::spawn(move || {
        while let Ok(state) = gate_notice.recv() {
            if matches!(state, TaskState::AwaitingApproval { .. }) {
                println!("  [闸门] 等待人工审批：{state:?}");
                // 提示完即退订：receiver 随循环结束而丢弃，
                // 之后机器推给它的快照被静默丢弃——观察方退场，机器照跑
                break;
            }
        }
        println!("  [闸门] 已退订，后续状态不再收到");
    });

    // ── 驱动方：执行工作流的 Agent，只发事件 ──
    let mut machine = flow.state_machine();

    println!("[Agent] 开工：code-review");
    machine.handle(&Event::Start);

    println!("[Agent] 第一步收尾");
    machine.handle(&Event::StepFinished); // → 闸门

    println!("[Agent] 人工审批通过");
    machine.handle(&Event::Approved(true)); // → 执行 release-audit

    println!("[Agent] 闸门步骤收尾");
    machine.handle(&Event::StepFinished); // → 完稿

    // ── 取：订阅是推，读终态依然是取 ──
    println!("[Agent] 终态：{:?}", machine.state());

    // 机器落地 → 发送端全部关闭 → 订阅者的接收循环自然退出（缓冲里的快照仍会取完）
    drop(machine);
    let _ = gate_handle.join();
    let _ = progress_handle.join();
    let _ = ledger_handle.join();
}
