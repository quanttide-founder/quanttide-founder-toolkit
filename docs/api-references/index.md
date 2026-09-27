# API 参考

两包同名同构：类型名一致、职责一致，行为经测试钉住。这组文档按 `core`、`memory`、`fiction` 三个模块给公开接口清单，签名按语言分标签，同页联动。

## 入口

```{tab-set}

:::{tab-item} Dart
:sync: dart

桶文件一个入口，接一次全拿到：

~~~dart
import 'package:quanttide_founder/quanttide_founder.dart';
~~~

:::

:::{tab-item} Rust
:sync: rust

crate 根重导出常用类型，域模块也可单独引：

~~~rust
use quanttide_founder::{MemoryRepository, InsightDoc, MemoryFlow};
use quanttide_founder::core::rules::Workflow;
~~~

:::
```

## 命名对应

- 类型与字段名两侧一致：`MemorySet`、`InsightItem.statement`、`Stage.number`
- 方法名 Dart 驼峰、Rust 蛇形：`decideMerge` 对 `decide_merge`，`allChapters` 对 `all_chapters`
- 错误通道不同：Dart 抛异常（`FileSystemException` 等），Rust 返回 `Result<T, Error>`
- LLM 接口不同：Dart 是 `LlmClient.complete(prompt)`，Rust 直接用 quanttide-agent 的 `LLM`
- memory 编排不同：Dart 是 `MemoryBloc` 实例方法，Rust 是 `MemoryFlow` 状态机加纯函数

## 模块地图

- [core](core.md)：`parse` 解析切分、`rules` 规则与工作流加载、`engine` 提取器与三个动词
- [memory](memory.md)：`models` 四层域模型、`repository` 记忆集装载、`states` 四步流程与判断规则
- [fiction](fiction.md)：`models` 小说与观察站、`repository` 装载、`states` 三步提炼与结构判断
- 错误类型不在单独页面：Dart 见各方法文档，Rust 见 `error` 模块的 `Error` 枚举
