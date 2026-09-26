/// memory 解析引擎演示：装载 `assets/memory` 并输出解析报告。
///
/// 用法：
/// ```sh
/// dart run example/parse_memory.dart            # 默认定位主仓库 assets/memory
/// dart run example/parse_memory.dart <仓库根路径>
/// ```
library;

import 'dart:io';

import 'package:quanttide_founder/quanttide_founder.dart';

void main(List<String> args) {
  final rootPath = args.isNotEmpty ? args.first : _defaultRoot();
  final repo = MemoryRepository.load(rootPath);

  print('memory 解析报告');
  print('根目录：$rootPath');
  for (final set in repo.sets) {
    _printSet(set);
  }

  final journals = repo.sets.fold(0, (sum, s) => sum + s.journals.length);
  final profiles = repo.sets.fold(0, (sum, s) => sum + s.profiles.length);
  final insights = repo.sets.fold(0, (sum, s) => sum + s.insights.length);
  final roadmaps = repo.sets.fold(0, (sum, s) => sum + s.roadmaps.length);
  print('');
  print(
    '合计：${repo.sets.length} 个记忆集 / $journals 篇日志 / '
    '$profiles 份档案 / $insights 份洞察 / $roadmaps 份路线图',
  );
}

void _printSet(MemorySet set) {
  print('');
  print('【${set.name}】');

  final journals = set.journals;
  if (journals.isNotEmpty) {
    final characters = journals.fold(0, (sum, j) => sum + j.characterCount);
    final newest = _formatDate(journals.first.date);
    final oldest = _formatDate(journals.last.date);
    print('  时间线：${journals.length} 篇（$oldest ~ $newest），$characters 字');
    for (final journal in journals) {
      final location = journal.source == JournalSource.root ? '集根' : 'journal';
      final preview = _clip(journal.segments.first.split('\n').first, 28);
      print(
        '    ${_formatDate(journal.date)}  $location  '
        '${journal.segments.length} 段  $preview',
      );
    }
  }

  if (set.profiles.isNotEmpty) {
    print('  档案：${set.profiles.length} 份');
    for (final profile in set.profiles) {
      final titles = profile.sections.map((s) => _clip(s.title, 10)).join('、');
      print(
        '    ${profile.title ?? profile.name}'
        '（${profile.sections.length} 节）：$titles',
      );
    }
  }

  if (set.insights.isNotEmpty) {
    print('  洞察：${set.insights.length} 份');
    for (final insight in set.insights) {
      final confirmed = insight.itemsOf(InsightGrade.confirmed).length;
      final hypothesis = insight.itemsOf(InsightGrade.hypothesis).length;
      final parts = <String>[];
      if (confirmed > 0) parts.add('已确认 $confirmed 条');
      if (hypothesis > 0) parts.add('假说 $hypothesis 条');
      if (parts.isNotEmpty) {
        print('    ${insight.title ?? insight.name}：${parts.join('，')}');
      } else {
        final prose = insight.sections.fold(
          0,
          (sum, s) => sum + s.paragraphs.length,
        );
        print('    ${insight.title ?? insight.name}：主题式 $prose 段');
      }
    }
  }

  if (set.roadmaps.isNotEmpty) {
    print('  路线图：${set.roadmaps.length} 份');
    for (final roadmap in set.roadmaps) {
      final parts = <String>[];
      if (roadmap.goal != null) {
        parts.add('目标「${_clip(roadmap.goal!, 20)}」');
      }
      void count(String label, int n) {
        if (n > 0) parts.add('$label $n');
      }

      count('元目标', roadmap.metaGoals.length);
      count('核心问题', roadmap.coreProblems.length);
      count('已决策', roadmap.decided.length);
      count('待决策', roadmap.pending.length);
      count('主题节', roadmap.themeSections.length);
      print('    ${roadmap.title ?? roadmap.name}：${parts.join('，')}');
    }
  }
}

String _formatDate(DateTime date) =>
    '${date.year}-${date.month.toString().padLeft(2, '0')}-'
    '${date.day.toString().padLeft(2, '0')}';

String _clip(String text, int width) {
  final flat = text.replaceAll('\n', ' ');
  return flat.length <= width ? flat : '${flat.substring(0, width)}…';
}

/// 依次尝试：脚本相对路径（主仓库）、当前目录，返回首个存在的候选。
String _defaultRoot() {
  final scriptDir = File.fromUri(Platform.script).parent.path;
  final candidates = [
    _normalize('$scriptDir/../../../../../assets/memory'),
    _normalize('${Directory.current.path}/assets/memory'),
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
