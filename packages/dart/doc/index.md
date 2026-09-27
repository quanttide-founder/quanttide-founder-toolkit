# quanttide_founder — Dart 包

量潮创始人工具箱的 Dart 实现。通用逻辑（读懂层定位、三层管线、规则设计）见 [toolkit 文档](../../../docs/index.md)。

## 目录结构

```
packages/dart/
├── assets/rules/          解析规则（profile.yaml 等）
├── lib/
│   ├── quanttide_founder.dart   包入口（导出全部公共 API）
│   └── src/
│       ├── memory_engine.dart         第一层 + 第二层
│       ├── semantic_extractor.dart    第三层
│       └── quanttide_founder_base.dart
├── doc/                   本文档
├── example/               演示入口
└── test/                  测试
```

## 类职责

| 类 | 文件 | 职责 |
|---|---|---|
| `MarkdownDocument` | memory_engine.dart | 语法解析：文本 → 块序列 |
| `splitSections` | memory_engine.dart | 结构切分：块序列 → Section 树 |
| `ParseRule` | semantic_extractor.dart | 加载 YAML 规则，生成自然语言指令 |
| `RuleBasedExtractor` | semantic_extractor.dart | 纯规则匹配，无 LLM 时的降级方案 |
| `LlmExtractor` | semantic_extractor.dart | LLM 按规则填表，首选方案 |
| `LlmClient` | semantic_extractor.dart | LLM 调用接口，由使用方实现 |

## 开发

```sh
dart pub get
dart analyze
dart test
```
