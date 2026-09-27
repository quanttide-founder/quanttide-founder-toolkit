/// 第一层：Markdown 语法解析。
///
/// 把文本拆成行级块序列（标题/段落/列表/分隔线），不做语义理解。
/// 任何 Markdown 文档通用，与具体业务域无关。
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
