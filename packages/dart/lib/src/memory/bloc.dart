/// memory/bloc：工作流编排（业务逻辑层）。
///
/// 事件进 → 跑 scan/judge/merge → 状态出。
library;

import '../core/engine.dart';
import '../core/rules.dart';
import 'models.dart';

/// memory 域的工作流编排。
class MemoryBloc {
  MemoryBloc({required this.engine});

  final Engine engine;

  /// 用指定 workflow 处理日志条目。
  WorkflowResult process(Workflow workflow, List<JournalEntry> entries) {
    return engine.run(workflow, entries);
  }
}
