/// 第二层：结构切分。
///
/// 按标题层级切节，提取通用节结构与命名条目。
/// 任何 Markdown 文档通用，与具体业务域无关。
library;

import 'markdown_parser.dart';

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
