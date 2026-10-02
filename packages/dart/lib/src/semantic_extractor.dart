/// 语义提取层：按规则文件说明「怎么读」，用 LLM 从 Section 树提取语义模型。
///
/// 与正则解析层的分工：
/// - 第一层（MarkdownDocument）：语法解析，识别标题/段落/列表/分隔线
/// - 第二层（splitSections）：结构切分，按标题层级切节
/// - 第三层（本文件）：语义提取，规则说明 + LLM，产出类型化模型
///
/// 规则文件位于 `tests/fixtures/rules/<type>.yaml`，描述字段来源与提取模式。
/// LLM 按规则填表，不自由发挥——规则说了提取什么字段、什么格式。
library;

import 'dart:convert';
import 'dart:io';

import 'package:yaml/yaml.dart';

import 'markdown_parser.dart';
import 'section_splitter.dart';

/// 解析规则：描述一种文档类型怎么读。
class ParseRule {
  ParseRule({
    required this.documentType,
    required this.title,
    required this.description,
    required this.sections,
  });

  final String documentType;
  final Map<String, dynamic> title;
  final Map<String, dynamic> description;
  final Map<String, dynamic> sections;

  /// 从 YAML 文件加载规则。
  static ParseRule fromFile(String path) {
    final content = File(path).readAsStringSync();
    final yaml = loadYaml(content) as Map;
    return ParseRule(
      documentType: yaml['document_type'] as String,
      title: Map<String, dynamic>.from(yaml['title'] as Map),
      description: Map<String, dynamic>.from(yaml['description'] as Map),
      sections: Map<String, dynamic>.from(yaml['sections'] as Map),
    );
  }

  /// 规则的自然语言描述，供 LLM 理解「怎么读」。
  String get instruction {
    final buf = StringBuffer();
    buf.writeln('文档类型: $documentType');
    buf.writeln('标题: ${title['source']}（兜底: ${title['fallback']}）');
    buf.writeln('说明: ${description['location']}，含义是${description['meaning']}');
    buf.writeln('章节切分: 按 ${sections['split_by']} 切分，${sections['subsections']} 为子节');

    final extract = sections['extract'] as List?;
    if (extract != null) {
      buf.writeln('提取字段:');
      for (final field in extract) {
        final map = Map<String, dynamic>.from(field as Map);
        buf.writeln('  - ${map['field']}: ${map['from']}');
        final patterns = map['patterns'] as List?;
        if (patterns != null) {
          buf.writeln('    格式: ${patterns.join(' / ')}');
        }
      }
    }

    final unknown = sections['unknown_content'];
    if (unknown != null) {
      buf.writeln('未知内容: $unknown');
    }

    return buf.toString();
  }
}

/// LLM 提取结果的通用结构。
class ExtractionResult {
  ExtractionResult({required this.title, this.description, required this.sections});

  final String? title;
  final String? description;
  final List<Map<String, dynamic>> sections;

  Map<String, dynamic> toJson() => {
    'title': title,
    'description': description,
    'sections': sections,
  };
}

/// 语义提取器：接收 Section 树 + 规则，产出语义模型。
///
/// 提取方式可插拔：
/// - [RuleBasedExtractor]：纯规则提取（无 LLM，降级方案）
/// - [LlmExtractor]：LLM 按规则填表（首选）
abstract class SemanticExtractor {
  ExtractionResult extract({
    required List<RawSection> sections,
    required ParseRule rule,
  });
}

/// 规则提取器：无 LLM 时的降级方案，按规则中的 patterns 做文本匹配。
class RuleBasedExtractor implements SemanticExtractor {
  const RuleBasedExtractor();

  @override
  ExtractionResult extract({
    required List<RawSection> sections,
    required ParseRule rule,
  }) {
    final resultSections = <Map<String, dynamic>>[];

    for (final raw in sections.skip(1)) {
      final section = <String, dynamic>{'name': raw.title};

      final paragraphs = raw.blocks
          .where((b) => b.type == BlockType.paragraph)
          .map((b) => b.text)
          .toList();
      if (paragraphs.isNotEmpty) section['paragraphs'] = paragraphs;

      final items = <Map<String, String>>[];
      for (final block in raw.blocks) {
        if (block.type == BlockType.bullet) {
          items.add(_parseItem(block.text));
        }
      }
      if (items.isNotEmpty) section['items'] = items;

      // 子节递归
      if (raw.subsections.isNotEmpty) {
        final subSections = <Map<String, dynamic>>[];
        for (final sub in raw.subsections) {
          final subMap = <String, dynamic>{'name': sub.title};
          final subParas = sub.blocks
              .where((b) => b.type == BlockType.paragraph)
              .map((b) => b.text)
              .toList();
          if (subParas.isNotEmpty) subMap['paragraphs'] = subParas;
          final subItems = <Map<String, String>>[];
          for (final block in sub.blocks) {
            if (block.type == BlockType.bullet) {
              subItems.add(_parseItem(block.text));
            }
          }
          if (subItems.isNotEmpty) subMap['items'] = subItems;
          subSections.add(subMap);
        }
        section['subsections'] = subSections;
      }

      resultSections.add(section);
    }

    // 提取标题与说明
    String? title;
    String? description;
    if (sections.isNotEmpty) {
      final first = sections.first;
      final firstParas = first.blocks
          .where((b) => b.type == BlockType.paragraph)
          .map((b) => b.text)
          .toList();
      if (firstParas.isNotEmpty) description = firstParas.join('\n');
    }

    return ExtractionResult(
      title: title,
      description: description,
      sections: resultSections,
    );
  }

  Map<String, String> _parseItem(String text) {
    final bold = RegExp(r'^\*\*(.+?)\*\*\s*[：:]\s*(.*)$').firstMatch(text);
    if (bold != null) return {'name': bold.group(1)!, 'detail': bold.group(2)!};

    final fullwidth = text.indexOf('：');
    final halfwidth = text.indexOf(':');
    final index = fullwidth >= 0 && (halfwidth < 0 || fullwidth < halfwidth)
        ? fullwidth
        : halfwidth;
    if (index > 0) {
      return {
        'name': text.substring(0, index).trim(),
        'detail': text.substring(index + 1).trim(),
      };
    }
    return {'name': text.trim(), 'detail': ''};
  }
}

/// LLM 提取器：把规则说明 + Section 树发给 LLM，按规则填表提取。
///
/// 需要提供一个 [LlmClient] 实现来完成实际调用。
class LlmExtractor implements SemanticExtractor {
  LlmExtractor({required this.client});

  final LlmClient client;

  @override
  ExtractionResult extract({
    required List<RawSection> sections,
    required ParseRule rule,
  }) {
    final prompt = _buildPrompt(sections, rule);
    final response = client.complete(prompt);
    return _parseResponse(response);
  }

  String _buildPrompt(List<RawSection> sections, ParseRule rule) {
    final buf = StringBuffer();
    buf.writeln('你是文档解析器。按以下规则从文档中提取结构化数据，只输出 JSON，不要解释。');
    buf.writeln();
    buf.writeln('## 解析规则');
    buf.writeln(rule.instruction);
    buf.writeln();
    buf.writeln('## 文档内容');

    for (final section in sections) {
      if (section.title.isNotEmpty) {
        buf.writeln('\n### ${section.title}');
      }
      for (final block in section.blocks) {
        switch (block.type) {
          case BlockType.paragraph:
            buf.writeln(block.text);
          case BlockType.bullet:
            buf.writeln('- ${block.text}');
          case BlockType.separator:
            buf.writeln('---');
          case BlockType.heading:
            buf.writeln('${'#' * block.level} ${block.text}');
        }
      }
      for (final sub in section.subsections) {
        buf.writeln('\n#### ${sub.title}');
        for (final block in sub.blocks) {
          if (block.type == BlockType.paragraph) buf.writeln(block.text);
          if (block.type == BlockType.bullet) buf.writeln('- ${block.text}');
        }
      }
    }

    buf.writeln();
    buf.writeln('## 输出格式');
    buf.writeln('{"title": "...", "description": "...", "sections": [{"name": "...", "paragraphs": [...], "items": [{"name": "...", "detail": "..."}]}]}');

    return buf.toString();
  }

  ExtractionResult _parseResponse(String response) {
    // 尝试从响应中提取 JSON
    final jsonMatch = RegExp(r'\{[\s\S]*\}').firstMatch(response);
    if (jsonMatch == null) {
      return ExtractionResult(title: null, description: null, sections: []);
    }
    final json = jsonDecode(jsonMatch.group(0)!);
    return ExtractionResult(
      title: json['title'] as String?,
      description: json['description'] as String?,
      sections: (json['sections'] as List?)?.cast<Map<String, dynamic>>() ?? [],
    );
  }
}

/// LLM 调用接口，由使用方实现。
abstract class LlmClient {
  String complete(String prompt);
}
