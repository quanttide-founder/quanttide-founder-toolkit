/// core/engine：语义提取 + 计算引擎。
///
/// 提取：按 Artifact 规则从 Section 树提取语义模型（LLM 填表 / 规则降级）。
/// 计算：scan / judge / merge 三个动词，workflow 用它们组装。
library;

import 'dart:convert';

import 'parse.dart';
import 'rules.dart';
import 'llm.dart';

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
    required Artifact rule,
  });
}

/// 规则提取器：无 LLM 时的降级方案，按规则中的 patterns 做文本匹配。
class RuleBasedExtractor implements SemanticExtractor {
  const RuleBasedExtractor();

  @override
  ExtractionResult extract({
    required List<RawSection> sections,
    required Artifact rule,
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
    required Artifact rule,
  }) {
    final prompt = _buildPrompt(sections, rule);
    final response = client.complete(prompt);
    return _parseResponse(response);
  }

  String _buildPrompt(List<RawSection> sections, Artifact rule) {
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

// ---------------------------------------------------------------------------
// 计算引擎：三个动词
// ---------------------------------------------------------------------------

/// 计算结果。
class WorkflowResult {
  WorkflowResult({required this.items, this.judgments = const []});

  final List<dynamic> items;
  final List<Judgment> judgments;
}

/// 判断结果。
class Judgment {
  Judgment({required this.decision, this.reason});

  final String decision;
  final String? reason;
}

/// 候选条目。
class Candidate {
  Candidate({required this.source, required this.text});

  final String source;
  final String text;
}

/// 计算引擎：scan / judge / merge 三个动词 + workflow 编排。
class Engine {
  Engine({required this.llmClient});

  final LlmClient llmClient;

  /// 编排：按 workflow 声明依次执行 steps。
  WorkflowResult run(Workflow workflow, List<dynamic> input) {
    var items = input;
    final judgments = <Judgment>[];

    for (final step in workflow.steps) {
      switch (step.verb) {
        case 'scan':
          final keywords = <String>[]; // 从 workflow.rules 解析或由调用方指定
          items = scan(items, keywords);
        case 'judge':
          final j = judge(workflow.rules, items);
          judgments.add(j);
          items = items; // judge 产出去向标注，不改 items
        case 'merge':
          items = merge(items, []);
        default:
          throw ArgumentError('未知动词: ${step.verb}（只认 scan / judge / merge）');
      }
    }

    return WorkflowResult(items: items, judgments: judgments);
  }

  /// 按关键词定位候选（代码）。
  List<Candidate> scan(List<dynamic> items, List<String> keywords) {
    final candidates = <Candidate>[];
    for (final item in items) {
      final text = item.toString();
      for (final kw in keywords) {
        if (text.contains(kw)) {
          candidates.add(Candidate(source: item.hashCode.toString(), text: text));
          break;
        }
      }
    }
    return candidates;
  }

  /// 让 LLM 按规则判断（LLM）。
  Judgment judge(String rules, List<dynamic> items) {
    final prompt = '按以下规则判断，只输出决策和原因：\n\n规则：$rules\n\n条目：$items';
    final response = llmClient.complete(prompt);
    return Judgment(decision: response.trim());
  }

  /// 按策略合并条目（代码）。
  List<dynamic> merge(List<dynamic> existing, List<dynamic> incoming) {
    return [...existing, ...incoming];
  }
}
