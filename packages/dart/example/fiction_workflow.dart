/// fiction 工作流演示：FictionBloc 的三步提炼、编号轴、阶段流转、包装文案。
///
/// 用法：
/// \`\`\`sh
/// dart run example/fiction_workflow.dart            # 默认定位主仓库 assets/fiction
/// dart run example/fiction_workflow.dart <路径>
/// \`\`\`
library;

import 'dart:io';

import 'package:quanttide_founder/quanttide_founder.dart';

void main(List<String> args) {
  final rootPath = args.isNotEmpty ? args.first : _defaultRoot();
  final repo = FictionRepository.load(rootPath);

  final engine = Engine(llmClient: _ConsoleLlm());
  final bloc = FictionBloc(engine: engine);

  print('fiction 工作流演示');
  print('根目录：$rootPath');

  // 三步提炼：观察站情绪日记 → 场景素材 + 母题卡片
  print('');
  print('═══ 三步提炼（观察站 → 创作片段）═══');
  for (final diary in repo.observation.emotionalDiaries) {
    final fragment = bloc.extract(diary);
    print('  来源：${fragment.source}');
    print('    母题：${fragment.motif}');
    print('    场景：${fragment.scene}');
    print('');
  }

  // 章节编号轴
  for (final novel in repo.novels) {
    print('═══ ${novel.name} ═══');

    // 编号轴：预留空号
    final gaps = bloc.findGaps(novel);
    if (gaps.isNotEmpty) {
      print('  预留空号：${gaps.join('、')}');
    }

    // 分配下一个编号
    final next = bloc.assignNumber(novel);
    print('  下一个可用编号：${next.assigned}${next.gap != null ? "（填空号 ${next.gap}）" : "（追加）"}');

    // 阶段流转
    print('  阶段流转：${bloc.stageFlow(novel).join(' → ')}');

    // 各阶段完成度
    for (final stage in novel.stages) {
      final status = bloc.checkStage(stage);
      final label = status.isComplete ? '✓' : '…';
      print('    $label ${stage.number}_${stage.name}：${status.totalChapters} 章'
          '${status.missingNumbers.isNotEmpty ? "，缺 ${status.missingNumbers.join('、')}" : ""}');
    }

    // 包装文案（取第一个定稿/成稿章节）
    for (final stage in novel.stages) {
      if (stage.name.contains('定稿') || stage.name.contains('成稿')) {
        if (stage.chapters.isNotEmpty) {
          final packaging = bloc.extractPackaging(stage.chapters.first.content);
          print('  包装文案（${stage.chapters.first.title}）：');
          print('    标题：${packaging['title']}');
          print('    简介：${packaging['tagline']}');
          print('    立意：${packaging['theme']}');
          break;
        }
      }
    }
    print('');
  }
}

class _ConsoleLlm implements LlmClient {
  @override
  String complete(String prompt) => '（演示模式，未调用 LLM）';
}

String _defaultRoot() {
  final scriptDir = File.fromUri(Platform.script).parent.path;
  final candidates = [
    '$scriptDir/../../../../../assets/fiction',
    '${Directory.current.path}/assets/fiction',
  ];
  for (final c in candidates) {
    if (Directory(c).existsSync()) return c;
  }
  return candidates.first;
}
