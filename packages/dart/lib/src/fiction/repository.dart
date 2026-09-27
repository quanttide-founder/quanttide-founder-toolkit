/// fiction/repository：仓库装载——发现小说与观察站、逐层读文件。
library;

import 'dart:io';

import 'models.dart';

class FictionRepository {
  FictionRepository._(this.root, this.novels, this.observation);

  final Directory root;
  final List<Novel> novels;
  final Observation observation;

  /// 装载仓库。小说目录 = 含 `index.md` 或 `N_阶段` 子目录的目录。
  static FictionRepository load(String rootPath) {
    final root = Directory(rootPath);
    if (!root.existsSync()) {
      throw FileSystemException('fiction 仓库根目录不存在', rootPath);
    }

    final novels = <Novel>[];
    Observation? observation;

    for (final entry in root.listSync().toList()
      ..sort((a, b) => a.path.compareTo(b.path))) {
      if (entry is! Directory) continue;
      final name = entry.path.split('/').last;
      if (name.startsWith('.')) continue;

      if (name == '观察站') {
        observation = _parseObservation(entry);
        continue;
      }
      if (name == '实验室') continue; // 实验室由使用方单独处理

      // 判断是否为小说目录：含 index.md 或 N_ 阶段子目录
      final hasIndex = File('${entry.path}/index.md').existsSync();
      final hasStage = entry.listSync().any(
        (e) => e is Directory && Stage.stageDir.hasMatch(e.path.split('/').last),
      );
      if (!hasIndex && !hasStage) continue;

      final stages = Stage.discover(entry);
      final indexFile = File('${entry.path}/index.md');
      novels.add(Novel(
        name: name,
        directory: entry,
        stages: stages,
        indexContent: indexFile.existsSync() ? indexFile.readAsStringSync() : null,
      ));
    }

    return FictionRepository._(
      root,
      novels,
      observation ?? Observation(emotionalDiaries: [], socialObservations: []),
    );
  }

  static Observation _parseObservation(Directory dir) {
    final diaries = <EmotionalDiary>[];
    final observations = <SocialObservation>[];

    void readMd(Directory d, void Function(String title, File file, String content) add) {
      if (!d.existsSync()) return;
      for (final entry in d.listSync()) {
        if (entry is! File) continue;
        final name = entry.path.split('/').last;
        if (!name.endsWith('.md') || name == 'README.md') continue;
        add(
          name.substring(0, name.length - 3),
          entry,
          entry.readAsStringSync(),
        );
      }
    }

    readMd(Directory('${dir.path}/1_情绪日记'), (title, file, content) {
      diaries.add(EmotionalDiary(title: title, file: file, content: content));
    });
    readMd(Directory('${dir.path}/2_社会观察'), (title, file, content) {
      observations.add(SocialObservation(title: title, file: file, content: content));
    });

    return Observation(emotionalDiaries: diaries, socialObservations: observations);
  }
}
