/// core/llm：LLM 调用接口，由使用方实现。
library;

/// LLM 调用接口，由使用方实现。
abstract class LlmClient {
  String complete(String prompt);
}
