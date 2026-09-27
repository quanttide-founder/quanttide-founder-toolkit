import 'package:quanttide_founder/quanttide_founder.dart';
import 'package:test/test.dart';

void main() {
  group('core/parse', () {
    test('MarkdownDocument 解析标题和段落', () {
      final doc = MarkdownDocument.parse('test.md', '# 标题\n\n段落内容');
      expect(doc.title, '标题');
    });

    test('splitSections 按 H2 切节', () {
      final doc = MarkdownDocument.parse('test.md', '前言\n\n## 一节\n\n内容\n\n## 二节');
      final sections = splitSections(doc.blocks, 2);
      expect(sections.length, 3); // 前言 + 两节
    });
  });

  group('core/rules', () {
    test('Workflow 加载 YAML', () {
      // 用内存内容测试——不依赖文件系统
    });
  });
}
