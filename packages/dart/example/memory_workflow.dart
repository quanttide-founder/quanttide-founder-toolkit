/// memory 工作流演示：MemoryBloc 的四步流程——扫描、过滤、聚类、合并。
///
/// 用法：
/// \`\`\`sh
/// dart run example/memory_workflow.dart            # 默认定位主仓库 assets/memory
/// dart run example/memory_workflow.dart <路径>
/// \`\`\`
library;

import 'dart:io';

import 'package:quanttide_founder/quanttide_founder.dart';

void main(List<String> args) {
  final rootPath = args.isNotEmpty ? args.first : _defaultRoot();
  final repo = MemoryRepository.load(rootPath);

  // MemoryBloc 用引擎编排工作流
  final engine = Engine(llmClient: _ConsoleLlm());
  final bloc = MemoryBloc(engine: engine);

  print('memory 工作流演示');
  print('根目录：$rootPath');

  for (final set in repo.sets) {
    if (set.journals.isEmpty) continue;
    print('');
    print('【${set.name}】');

    // 第一步：扫描——从日志里定位候选
    final texts = <String>[];
    for (final journal in set.journals) {
      for (final segment in journal.segments) {
        for (final line in segment.split('\n')) {
          if (line.trim().isNotEmpty) texts.add(line.trim());
        }
      }
    }

    // 第二步：过滤——三问路由
    final classified = bloc.classify(texts);
    final byDest = <Destination, int>{};
    for (final c in classified) {
      byDest[c.destination] = (byDest[c.destination] ?? 0) + 1;
    }

    print('  扫描：${texts.length} 行文本');
    print('  分类：');
    for (final entry in byDest.entries) {
      print('    ${entry.key.name} → ${entry.value} 条');
    }

    // 第三步：证据分级（示例：对前 3 条认知类条目分级）
    final insights = classified.where((c) => c.destination == Destination.insight).take(3).toList();
    if (insights.isNotEmpty) {
      print('  分级（前 ${insights.length} 条认知类）：');
      for (final item in insights) {
        final grade = bloc.grade(item.text);
        print('    [${grade.name}] ${item.text.substring(0, item.text.length.clamp(0, 30))}…');
      }
    }

    // 第四步：合并策略（示例：两条相似文本）
    if (insights.length >= 2) {
      final a = InsightItem(statement: insights[0].text, detail: '');
      final b = InsightItem(statement: insights[1].text, detail: '');
      final decision = bloc.decideMerge(a, b);
      print('  合并：${decision.action.name}（第一条 × 第二条）');
    }
  }
}

/// 演示用的 LLM 客户端——直接返回固定文本，不调用真实 API。
class _ConsoleLlm implements LlmClient {
  @override
  String complete(String prompt) => '（演示模式，未调用 LLM）';
}

String _defaultRoot() {
  final scriptDir = File.fromUri(Platform.script).parent.path;
  final candidates = [
    '$scriptDir/../../../../../assets/memory',
    '${Directory.current.path}/assets/memory',
  ];
  for (final c in candidates) {
    if (Directory(c).existsSync()) return c;
  }
  return candidates.first;
}
