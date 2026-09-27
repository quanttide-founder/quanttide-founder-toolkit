/// memory/repository：仓库装载——发现记忆集、逐层读文件。
library;

import 'dart:io';

import 'models.dart';

final _dateFile = RegExp(r'^(\d{4})-(\d{2})-(\d{2})\.md\$');

class MemorySet {
  MemorySet({
    required this.name,
    required this.root,
    required this.journals,
    required this.profiles,
    required this.insights,
    required this.roadmaps,
  });

  final String name;
  final Directory root;
  final List<JournalEntry> journals; // 按日期倒序
  final List<ProfileDoc> profiles;
  final List<InsightDoc> insights;
  final List<RoadmapDoc> roadmaps;
}

class MemoryRepository {
  MemoryRepository._(this.root, this.sets);

  final Directory root;
  final List<MemorySet> sets;

  static const _layerDirs = ['journal', 'profile', 'insight', 'roadmap'];

  /// 装载仓库：根目录下含任一层目录的目录视为一个记忆集。
  static MemoryRepository load(String rootPath) {
    final root = Directory(rootPath);
    if (!root.existsSync()) {
      throw FileSystemException('memory 仓库根目录不存在', rootPath);
    }
    final sets = <MemorySet>[];
    final entries = root.listSync().toList()
      ..sort((a, b) => a.path.compareTo(b.path));
    for (final entry in entries) {
      if (entry is! Directory) continue;
      final name = entry.path.split('/').last;
      if (name.startsWith('.')) continue;
      final isSet = _layerDirs.any(
        (layer) => Directory('${entry.path}/$layer').existsSync(),
      );
      if (isSet) sets.add(_parseSet(entry, name));
    }
    return MemoryRepository._(root, sets);
  }

  static MemorySet _parseSet(Directory setRoot, String name) {
    final journals = <JournalEntry>[];
    void readJournals(Directory dir, JournalSource source) {
      if (!dir.existsSync()) return;
      for (final entry
          in dir.listSync().toList()
            ..sort((a, b) => a.path.compareTo(b.path))) {
        if (entry is! File) continue;
        final match = _dateFile.firstMatch(entry.path.split('/').last);
        if (match == null) continue;
        journals.add(
          JournalEntry(
            date: DateTime.parse(
              '${match.group(1)}-${match.group(2)}-${match.group(3)}',
            ),
            path: entry.path,
            source: source,
            content: entry.readAsStringSync(),
          ),
        );
      }
    }

    readJournals(setRoot, JournalSource.root); // 集根的当天日志
    readJournals(Directory('${setRoot.path}/journal'), JournalSource.archive);
    journals.sort((a, b) => b.date.compareTo(a.date));

    List<File> layerFiles(String layer) {
      final dir = Directory('${setRoot.path}/$layer');
      if (!dir.existsSync()) return const [];
      final files =
          dir
              .listSync()
              .whereType<File>()
              .where(
                (f) =>
                    f.path.endsWith('.md') &&
                    f.path.split('/').last != 'README.md',
              )
              .toList()
            ..sort((a, b) => a.path.compareTo(b.path));
      return files;
    }

    return MemorySet(
      name: name,
      root: setRoot,
      journals: journals,
      profiles: layerFiles('profile').map((f) => ProfileDoc.parse(f)).toList(),
      insights: layerFiles('insight').map((f) => InsightDoc.parse(f)).toList(),
      roadmaps: layerFiles('roadmap').map((f) => RoadmapDoc.parse(f)).toList(),
    );
  }

  /// 全部记忆集的日志时间线，按日期倒序。
  List<JournalEntry> get allJournals =>
      sets.expand((s) => s.journals).toList()
        ..sort((a, b) => b.date.compareTo(a.date));
}


