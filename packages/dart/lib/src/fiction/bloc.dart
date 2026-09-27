/// fiction/bloc：工作流编排（业务逻辑层）。
library;

import '../core/engine.dart';
import '../core/rules.dart';
import 'models.dart';

/// fiction 域的工作流编排。
class FictionBloc {
  FictionBloc({required this.engine});

  final Engine engine;

  /// 用指定 workflow 处理章节。
  WorkflowResult process(Workflow workflow, List<Chapter> chapters) {
    return engine.run(workflow, chapters);
  }
}
