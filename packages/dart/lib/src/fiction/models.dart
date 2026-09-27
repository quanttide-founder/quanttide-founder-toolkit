/// fiction/models：域模型（纯数据，不读文件）。
library;

import 'dart:io';


// 通用解析层由使用方按需导入，本文件只依赖 dart:io。

// ---------------------------------------------------------------------------
// 创作阶段
// ---------------------------------------------------------------------------

/// 创作阶段目录：`{N}_{名称}` 模式动态识别，不硬编码阶段名。
///
/// 各小说阶段名不同（职场=灵感/场景/初稿/改稿/定稿，校园=素材/提纲/初稿/改稿），
/// 按编号前缀发现阶段，编号即流程顺序。
class Stage {
  Stage({required this.number, required this.name, required this.directory});

  final int number; // 阶段编号（1-5），来自目录名前缀
  final String name; // 阶段名称（灵感/场景/初稿/改稿/定稿…），来自目录名后缀
  final Directory directory;
  final List<Chapter> chapters = [];

  static final stageDir = RegExp(r'^(\d+)_(.+)$');

  /// 从目录条目发现阶段。返回按编号排序的阶段列表。
  static List<Stage> discover(Directory novelRoot) {
    final stages = <Stage>[];
    for (final entry in novelRoot.listSync()) {
      if (entry is! Directory) continue;
      final name = entry.path.split('/').last;
      final match = stageDir.firstMatch(name);
      if (match == null) continue;
      final stage = Stage(
        number: int.parse(match.group(1)!),
        name: match.group(2)!,
        directory: entry,
      );
      stage.chapters.addAll(Chapter.discover(entry));
      stages.add(stage);
    }
    stages.sort((a, b) => a.number.compareTo(b.number));
    return stages;
  }
}

// ---------------------------------------------------------------------------
// 章节
// ---------------------------------------------------------------------------

/// 章节文件：`{序号}_{标题}.md`。
///
/// 编号语义：
/// - 序号对应正文最终阅读顺序，各阶段共用同一编号轴
/// - 预留空号（宁空勿移），同一序号可有多份（初稿/改稿/定稿）
/// - 未编号文件（如「地摊火锅.md」）是替代草稿，不占编号
/// - `0_` 前缀是非正文文件（前言等），不占正文章节号
class Chapter {
  Chapter({this.number, required this.title, required this.file, required this.content});

  final int? number; // 章节序号，null 表示未编号（替代草稿）
  final String title; // 标题（文件名去掉序号前缀与扩展名）
  final File file;
  final String content;

  bool get isPreface => number == 0; // 前言等非正文
  bool get isUnnumbered => number == null; // 未编号的替代草稿

  static final _chapterFile = RegExp(r'^(?:(\d+)_)?(.+)\.md$');

  /// 从阶段目录发现章节。跳过 README。
  static List<Chapter> discover(Directory stageDir) {
    final chapters = <Chapter>[];
    for (final entry in stageDir.listSync()) {
      if (entry is! File) continue;
      final name = entry.path.split('/').last;
      if (name == 'README.md') continue;
      if (!name.endsWith('.md')) continue;
      final match = _chapterFile.firstMatch(name);
      if (match == null) continue;
      chapters.add(Chapter(
        number: match.group(1) != null ? int.parse(match.group(1)!) : null,
        title: match.group(2)!,
        file: entry,
        content: entry.readAsStringSync(),
      ));
    }
    chapters.sort((a, b) => (a.number ?? 9999).compareTo(b.number ?? 9999));
    return chapters;
  }
}

// ---------------------------------------------------------------------------
// 小说
// ---------------------------------------------------------------------------

/// 小说：一个创作目录，含 index.md（晋江资料）与若干创作阶段。
class Novel {
  Novel({required this.name, required this.directory, required this.stages, this.indexContent});

  final String name; // 目录名（职场言情/校园言情/重生言情）
  final Directory directory;
  final List<Stage> stages;
  final String? indexContent; // index.md 原文，语义提取由 ParseRule + LLM 完成

  /// 全部章节（跨阶段），按编号排序。
  Iterable<Chapter> get allChapters =>
      stages.expand((s) => s.chapters).toList()
        ..sort((a, b) => (a.number ?? 9999).compareTo(b.number ?? 9999));

  /// 按阶段名查找。
  Stage? stageByName(String name) {
    for (final s in stages) {
      if (s.name == name) return s;
    }
    return null;
  }
}

// ---------------------------------------------------------------------------
// 观察站
// ---------------------------------------------------------------------------

/// 情绪日记条目。
class EmotionalDiary {
  EmotionalDiary({required this.title, required this.file, required this.content});

  final String title;
  final File file;
  final String content;
}

/// 社会观察条目。
class SocialObservation {
  SocialObservation({required this.title, required this.file, required this.content});

  final String title;
  final File file;
  final String content;
}

/// 观察站：跨系列共用的观察素材（情绪日记 + 社会观察）。
///
/// 情绪日记是母题的发现来源；创作日志/创作谈/创作设定已迁至 memory 仓库的 write/ 记忆集。
class Observation {
  Observation({required this.emotionalDiaries, required this.socialObservations});

  final List<EmotionalDiary> emotionalDiaries;
  final List<SocialObservation> socialObservations;
}

// ---------------------------------------------------------------------------
// 仓库装载
// ---------------------------------------------------------------------------

/// fiction 仓库：发现小说与观察站，逐层装载。
///
/// 仓库结构：
/// ```
/// fiction/
/// ├── {小说名}/           每部小说一个目录
/// │   ├── index.md        晋江发文资料
/// │   ├── {N}_{阶段}/     创作阶段（N_灵感/2_场景/…）
/// │   └── README.md
/// ├── 观察站/
/// │   ├── 1_情绪日记/
/// │   └── 2_社会观察/
/// └── 实验室/
/// ```
