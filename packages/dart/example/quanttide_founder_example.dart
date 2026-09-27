/// 基本示例：解析 + 计算。
library;

import 'package:quanttide_founder/quanttide_founder.dart';

void main() {
  // 语法解析
  final doc = MarkdownDocument.parse('demo.md', '# 标题\n\n段落内容');
  print('标题: ${doc.title}');

  // 结构切分
  final sections = splitSections(doc.blocks, 2);
  print('章节数: ${sections.length}');
}
