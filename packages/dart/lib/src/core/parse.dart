/// core/parse：Markdown 解析 + 结构切分。
///
/// 文本 → 块序列 → Section 树。通用，不认业务。
library;

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
