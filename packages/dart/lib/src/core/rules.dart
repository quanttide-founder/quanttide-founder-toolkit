/// core/rules：Artifact（名词定义）与 Workflow（动词定义）的 YAML 加载。
///
/// Artifact 定义一个语义模型长什么样、从哪提取。
/// Workflow 定义一个任务怎么做，用引擎的三个动词（scan/judge/merge）组装。
library;

import 'dart:io';

import 'package:yaml/yaml.dart';

class Artifact {
  Artifact({
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
  static Artifact fromFile(String path) {
    final content = File(path).readAsStringSync();
    final yaml = loadYaml(content) as Map;
    return Artifact(
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

/// Workflow：一个任务怎么做，由 steps 组装。
class Workflow {
  Workflow({
    required this.name,
    required this.description,
    required this.input,
    required this.output,
    required this.steps,
    required this.rules,
  });

  final String name;
  final String description;
  final String input;
  final List<String> output;
  final List<Step> steps;
  final String rules;

  static Workflow fromFile(String path) {
    final content = File(path).readAsStringSync();
    final yaml = loadYaml(content) as Map;
    final rawSteps = (yaml['steps'] as List?) ?? [];
    return Workflow(
      name: yaml['name'] as String,
      description: yaml['description'] as String? ?? '',
      input: yaml['input'] as String,
      output: (yaml['output'] as List?)?.cast<String>() ?? [],
      steps: rawSteps.map((s) {
        final map = Map<String, dynamic>.from(s as Map);
        final verb = map.keys.first;
        return Step(verb: verb, description: map[verb] as String);
      }).toList(),
      rules: yaml['rules'] as String? ?? '',
    );
  }
}

/// Step：workflow 的一个步骤，verb 只能是 scan / judge / merge。
class Step {
  Step({required this.verb, required this.description});

  final String verb;
  final String description;
}
