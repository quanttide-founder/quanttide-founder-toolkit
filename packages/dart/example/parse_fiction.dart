/// fiction 解析引擎演示：装载 fiction 仓库并输出解析报告。
///
/// 用法：
/// ```sh
/// dart run example/parse_fiction.dart            # 默认定位主仓库 assets/fiction
/// dart run example/parse_fiction.dart <仓库根路径>
/// ```
library;

import 'dart:io';

import 'package:quanttide_founder/quanttide_founder.dart';

void main(List<String> args) {
  final rootPath = args.isNotEmpty ? args.first : _defaultRoot();
  final repo = FictionRepository.load(rootPath);

  print('fiction 解析报告');
  print('根目录：$rootPath');

  for (final novel in repo.novels) {
    _printNovel(novel);
  }

  _printObservation(repo.observation);

  final totalChapters = repo.novels.fold(0, (sum, n) => sum + n.allChapters.length);
  print('');
  print(
    '合计：${repo.novels.length} 部小说 / $totalChapters 个章节 / '
    '${repo.observation.emotionalDiaries.length} 篇情绪日记 / '
    '${repo.observation.socialObservations.length} 条社会观察',
  );
}

void _printNovel(Novel novel) {
  print('');
  print('【${novel.name}】');

  // 晋江资料（index.md）
  if (novel.indexContent != null) {
    final title = _extractTitle(novel.indexContent!);
    print('  晋江资料：${title ?? '（未提取标题）'}');
  }

  // 创作阶段
  for (final stage in novel.stages) {
    final numbered = stage.chapters.where((c) => c.number != null).toList();
    final unnumbered = stage.chapters.where((c) => c.number == null).toList();
    final prefaces = numbered.where((c) => c.isPreface).toList();

    final parts = <String>[];
    if (numbered.isNotEmpty) {
      final numbers = numbered.map((c) => c.number).toList()..sort();
      parts.add('${numbered.length} 章（编号 ${numbers.first}–${numbers.last}）');
    }
    if (prefaces.isNotEmpty) parts.add('${prefaces.length} 篇前言');
    if (unnumbered.isNotEmpty) parts.add('${unnumbered.length} 篇未编号');

    print('  ${stage.number}_${stage.name}：${parts.join('，')}');

    for (final chapter in stage.chapters) {
      final label = chapter.isPreface
          ? '${chapter.number}_'
          : chapter.number != null
              ? '${chapter.number}_'
              : '未编号 ';
      final chars = chapter.content.replaceAll(RegExp(r'\s'), '').length;
      print('    $label${chapter.title}（$chars 字）');
    }
  }

  // 编号覆盖情况
  final allNumbers = novel.allChapters
      .where((c) => c.number != null && !c.isPreface)
      .map((c) => c.number!)
      .toSet();
  if (allNumbers.isNotEmpty) {
    final max = allNumbers.reduce((a, b) => a > b ? a : b);
    final gaps = <int>[];
    for (var i = 1; i <= max; i++) {
      if (!allNumbers.contains(i)) gaps.add(i);
    }
    if (gaps.isNotEmpty) {
      print('  预留空号：${gaps.join('、')}');
    }
  }
}

void _printObservation(Observation obs) {
  if (obs.emotionalDiaries.isEmpty && obs.socialObservations.isEmpty) return;

  print('');
  print('【观察站】');

  if (obs.emotionalDiaries.isNotEmpty) {
    print('  情绪日记：${obs.emotionalDiaries.length} 篇');
    for (final diary in obs.emotionalDiaries) {
      final chars = diary.content.replaceAll(RegExp(r'\s'), '').length;
      print('    ${diary.title}（$chars 字）');
    }
  }

  if (obs.socialObservations.isNotEmpty) {
    print('  社会观察：${obs.socialObservations.length} 条');
    for (final obs2 in obs.socialObservations) {
      final chars = obs2.content.replaceAll(RegExp(r'\s'), '').length;
      print('    ${obs2.title}（$chars 字）');
    }
  }
}

String? _extractTitle(String indexContent) {
  for (final line in indexContent.split('\n')) {
    final trimmed = line.trim();
    if (trimmed.startsWith('# ')) return trimmed.substring(2);
  }
  return null;
}

/// 依次尝试：脚本相对路径（主仓库）、当前目录，返回首个存在的候选。
String _defaultRoot() {
  final scriptDir = File.fromUri(Platform.script).parent.path;
  final candidates = [
    _normalize('$scriptDir/../../../../../assets/fiction'),
    _normalize('${Directory.current.path}/assets/fiction'),
  ];
  for (final candidate in candidates) {
    if (Directory(candidate).existsSync()) return candidate;
  }
  return candidates.first;
}

String _normalize(String path) {
  final absolute = path.startsWith('/')
      ? path
      : '${Directory.current.path}/$path';
  final parts = <String>[];
  for (final segment in absolute.split('/')) {
    if (segment.isEmpty || segment == '.') continue;
    if (segment == '..' && parts.isNotEmpty && parts.last != '..') {
      parts.removeLast();
      continue;
    }
    parts.add(segment);
  }
  return '/${parts.join('/')}';
}
