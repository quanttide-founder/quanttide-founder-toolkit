/// memory 解析引擎：解析 `assets/memory` 记忆仓库的四层结构。
///
/// 分三层：
/// 1. 通用 Markdown 块解析：[MarkdownDocument] 把文本拆成行级块序列；
/// 2. 类型化解析：在块序列上提取时间线、档案、洞察、路线图的语义模型；
/// 3. 仓库装载：[MemoryRepository] 发现记忆集并逐层装载为 [MemorySet]。
library;

import 'dart:io';

// ---------------------------------------------------------------------------
// 通用 Markdown 块解析
// ---------------------------------------------------------------------------

enum BlockType { heading, bullet, paragraph, separator }

class Block {
  const Block(this.type, {this.level = 0, this.text = ''});

  final BlockType type;
  final int level; // 仅 heading 有效：1-6
  final String text; // heading 去掉 # 后的文本；bullet 去掉标记后的文本；paragraph 原文
}

class MarkdownDocument {
  MarkdownDocument(this.path, this.blocks);

  final String path;
  final List<Block> blocks;

  static final _heading = RegExp(r'^(#{1,6})\s+(.*?)\s*#*\s*$');
  static final _bullet = RegExp(r'^(?:[-*]|\d+\.)\s+(.*)$');
  static final _separator = RegExp(r'^-{2,}\s*$');

  static MarkdownDocument parse(String path, String content) {
    final blocks = <Block>[];
    final para = StringBuffer();

    void flush() {
      if (para.isNotEmpty) {
        blocks.add(Block(BlockType.paragraph, text: para.toString().trim()));
        para.clear();
      }
    }

    for (final raw in content.split(RegExp(r'\r?\n'))) {
      final line = raw.trim();
      if (line.isEmpty) {
        flush();
        continue;
      }
      final heading = _heading.firstMatch(line);
      if (heading != null) {
        flush();
        blocks.add(
          Block(
            BlockType.heading,
            level: heading.group(1)!.length,
            text: heading.group(2)!,
          ),
        );
        continue;
      }
      if (_separator.hasMatch(line)) {
        flush();
        blocks.add(const Block(BlockType.separator));
        continue;
      }
      final bullet = _bullet.firstMatch(line);
      if (bullet != null) {
        flush();
        blocks.add(Block(BlockType.bullet, text: bullet.group(1)!));
        continue;
      }
      if (para.isEmpty) {
        para.write(line);
      } else {
        para.write('\n$line');
      }
    }
    flush();
    return MarkdownDocument(path, blocks);
  }

  /// 首个一级标题，无则返回 null。
  String? get title {
    for (final block in blocks) {
      if (block.type == BlockType.heading && block.level == 1) {
        return block.text;
      }
    }
    return null;
  }
}

// ---------------------------------------------------------------------------
// 分节：按指定层级的标题切分，下一层级标题成为子节
// ---------------------------------------------------------------------------

class RawSection {
  RawSection(this.level, this.title);

  final int level;
  final String title;
  final List<Block> blocks = [];
  final List<RawSection> subsections = [];
}

/// 以 `level` 级标题为边界切分；首个元素是首节之前的前言（title 为空串）。
List<RawSection> splitSections(List<Block> blocks, int level) {
  final sections = <RawSection>[RawSection(level, '')];
  var current = sections.first;
  RawSection? subsection;
  for (final block in blocks) {
    if (block.type == BlockType.heading && block.level == level) {
      current = RawSection(level, block.text);
      sections.add(current);
      subsection = null;
    } else if (block.type == BlockType.heading && block.level == level + 1) {
      subsection = RawSection(level + 1, block.text);
      current.subsections.add(subsection);
    } else {
      (subsection ?? current).blocks.add(block);
    }
  }
  return sections;
}

/// 通用节：主题段落 + 条目 + 子节，供档案与路线图主题节复用。
class TextSection {
  TextSection(this.title);

  final String title;
  final List<String> paragraphs = [];
  final List<String> bullets = [];
  final List<TextSection> subsections = [];

  static TextSection fromRaw(RawSection raw) {
    final section = TextSection(raw.title);
    for (final block in raw.blocks) {
      if (block.type == BlockType.paragraph) {
        section.paragraphs.add(block.text);
      } else if (block.type == BlockType.bullet) {
        section.bullets.add(block.text);
      }
    }
    for (final child in raw.subsections) {
      section.subsections.add(fromRaw(child));
    }
    return section;
  }
}

/// 把 `- **名称**：详情` 解析为结构化条目；无加粗时以首个全角/半角冒号切分。
NamedItem parseNamedItem(String text) {
  final bold = RegExp(r'^\*\*(.+?)\*\*\s*[：:]\s*(.*)$').firstMatch(text);
  if (bold != null) {
    return NamedItem(bold.group(1)!, bold.group(2)!);
  }
  final fullwidth = text.indexOf('：');
  final halfwidth = text.indexOf(':');
  final index = fullwidth >= 0 && (halfwidth < 0 || fullwidth < halfwidth)
      ? fullwidth
      : halfwidth;
  if (index > 0) {
    return NamedItem(
      text.substring(0, index).trim(),
      text.substring(index + 1).trim(),
    );
  }
  return NamedItem(text.trim(), '');
}

class NamedItem {
  NamedItem(this.name, this.detail);

  final String name;
  final String detail;
}

// ---------------------------------------------------------------------------
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
