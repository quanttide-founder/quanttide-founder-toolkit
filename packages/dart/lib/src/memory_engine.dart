/// memory 域：记忆仓库的模型与装载。
///
/// 解析 `assets/memory` 记忆仓库的四层结构（journal / profile / insight / roadmap）。
/// 通用解析层见 markdown_parser.dart 与 section_splitter.dart，
/// 语义提取层见 semantic_extractor.dart。
library;

import 'dart:io';

import 'markdown_parser.dart';
import 'section_splitter.dart';

// 时间线：日志
// ---------------------------------------------------------------------------

enum JournalSource { root, archive }

class JournalEntry {
  JournalEntry({
    required this.date,
    required this.path,
    required this.source,
    required this.content,
  });

  final DateTime date;
  final String path;
  final JournalSource source;
  final String content;

  /// 按 `---` 类分隔线切出的会话段。
  late final List<String> segments = content
      .split(RegExp(r'^[ \t]*-{2,}[ \t]*$', multiLine: true))
      .map((segment) => segment.trim())
      .where((segment) => segment.isNotEmpty)
      .toList();

  int get characterCount => content.replaceAll(RegExp(r'\s'), '').length;
}

final _dateFile = RegExp(r'^(\d{4})-(\d{2})-(\d{2})\.md$');

// ---------------------------------------------------------------------------
// 特征层：个人档案
// ---------------------------------------------------------------------------

class ProfileDoc {
  ProfileDoc({
    required this.path,
    required this.name,
    required this.title,
    required this.description,
    required this.sections,
  });

  final String path;
  final String name;
  final String? title; // 一级标题，缺失时由文件名兜底
  final String description; // 标题与首个二级标题之间的来源说明
  final List<TextSection> sections;

  static ProfileDoc parse(File file) {
    final name = _baseName(file);
    final doc = MarkdownDocument.parse(file.path, file.readAsStringSync());
    final raw = splitSections(doc.blocks, 2);
    return ProfileDoc(
      path: file.path,
      name: name,
      title: doc.title,
      description: raw.first.blocks
          .where((b) => b.type == BlockType.paragraph)
          .map((b) => b.text)
          .join('\n'),
      sections: raw.skip(1).map(TextSection.fromRaw).toList(),
    );
  }
}

// ---------------------------------------------------------------------------
// 认知层：洞察
// ---------------------------------------------------------------------------

enum InsightGrade { confirmed, hypothesis }

class InsightItem {
  InsightItem({required this.statement, required this.detail, this.evidence});

  final String statement; // 命题
  final String detail; // 解释
  final String? evidence; // 「依据：」之后的证据
}

class InsightSection {
  InsightSection({required this.title, required this.grade});

  String title;
  final InsightGrade? grade; // 已确认/假说分级；主题式节为 null
  final List<InsightItem> items = [];
  final List<String> paragraphs = [];
}

class InsightDoc {
  InsightDoc({
    required this.path,
    required this.name,
    required this.title,
    required this.sections,
  });

  final String path;
  final String name;
  final String? title;
  final List<InsightSection> sections;

  Iterable<InsightItem> itemsOf(InsightGrade grade) =>
      sections.where((s) => s.grade == grade).expand((s) => s.items);

  static InsightDoc parse(File file) {
    final name = _baseName(file);
    final doc = MarkdownDocument.parse(file.path, file.readAsStringSync());
    final raw = splitSections(doc.blocks, 2);
    final parsed = <InsightSection>[];

    InsightSection build(RawSection section) {
      final title = section.title;
      final grade = title.contains('已确认')
          ? InsightGrade.confirmed
          : title.contains('假说')
          ? InsightGrade.hypothesis
          : null;
      final built = InsightSection(title: title, grade: grade);
      for (final block in section.blocks) {
        if (block.type == BlockType.bullet) {
          built.items.add(_parseInsightItem(block.text));
        } else if (block.type == BlockType.paragraph) {
          built.paragraphs.add(block.text);
        }
      }
      return built;
    }

    if (raw.length == 1) {
      // 无二级标题：全文作为一节（主题式散文）。
      final built = build(raw.first);
      if (built.title.isEmpty) built.title = doc.title ?? name;
      parsed.add(built);
    } else {
      parsed.addAll(raw.skip(1).map(build));
    }
    return InsightDoc(
      path: file.path,
      name: name,
      title: doc.title,
      sections: parsed,
    );
  }

  static InsightItem _parseInsightItem(String text) {
    final named = parseNamedItem(text);
    var detail = named.detail;
    String? evidence;
    final marker = detail.indexOf('依据：');
    if (marker >= 0) {
      evidence = detail.substring(marker + 3).trim();
      detail = detail.substring(0, marker).trim();
    }
    return InsightItem(
      statement: named.name,
      detail: detail,
      evidence: evidence,
    );
  }
}

// ---------------------------------------------------------------------------
// 方向层：路线图
// ---------------------------------------------------------------------------

class RoadmapDoc {
  RoadmapDoc({
    required this.path,
    required this.name,
    required this.title,
    required this.goal,
    required this.metaGoals,
    required this.coreProblems,
    required this.decided,
    required this.pending,
    required this.themeSections,
  });

  final String path;
  final String name;
  final String? title;
  final String? goal; // 「## 目标」的正文
  final List<NamedItem> metaGoals; // 「### 元目标」
  final List<NamedItem> coreProblems;
  final List<NamedItem> decided;
  final List<NamedItem> pending;

  /// 非方向层结构的主题节（写作集路线图为主题式）。
  final List<TextSection> themeSections;

  static RoadmapDoc parse(File file) {
    final name = _baseName(file);
    final doc = MarkdownDocument.parse(file.path, file.readAsStringSync());
    final raw = splitSections(doc.blocks, 2);

    String? goal;
    var metaGoals = <NamedItem>[];
    var coreProblems = <NamedItem>[];
    var decided = <NamedItem>[];
    var pending = <NamedItem>[];
    final themeSections = <TextSection>[];

    for (final section in raw.skip(1)) {
      final paragraphs = section.blocks
          .where((b) => b.type == BlockType.paragraph)
          .map((b) => b.text)
          .join('\n');
      switch (section.title) {
        case '目标':
          goal = paragraphs.isEmpty ? null : paragraphs;
          metaGoals = section.subsections
              .where((s) => s.title == '元目标')
              .expand((s) => s.blocks)
              .where((b) => b.type == BlockType.bullet)
              .map((b) => parseNamedItem(b.text))
              .toList();
        case '核心问题':
          coreProblems = _namedBullets(section);
        case '已决策':
          decided = _namedBullets(section);
        case '待决策':
          pending = _namedBullets(section);
        default:
          themeSections.add(TextSection.fromRaw(section));
      }
    }

    return RoadmapDoc(
      path: file.path,
      name: name,
      title: doc.title,
      goal: goal,
      metaGoals: metaGoals,
      coreProblems: coreProblems,
      decided: decided,
      pending: pending,
      themeSections: themeSections,
    );
  }

  static List<NamedItem> _namedBullets(RawSection section) => section.blocks
      .where((b) => b.type == BlockType.bullet)
      .map((b) => parseNamedItem(b.text))
      .toList();
}

// ---------------------------------------------------------------------------
// 仓库装载
// ---------------------------------------------------------------------------

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
      profiles: layerFiles('profile').map(ProfileDoc.parse).toList(),
      insights: layerFiles('insight').map(InsightDoc.parse).toList(),
      roadmaps: layerFiles('roadmap').map(RoadmapDoc.parse).toList(),
    );
  }

  /// 全部记忆集的日志时间线，按日期倒序。
  List<JournalEntry> get allJournals =>
      sets.expand((s) => s.journals).toList()
        ..sort((a, b) => b.date.compareTo(a.date));
}

String _baseName(File file) {
  final name = file.path.split('/').last;
  return name.endsWith('.md') ? name.substring(0, name.length - 3) : name;
}
