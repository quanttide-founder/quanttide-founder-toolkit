# 快速开始

两包当前发布到 `0.1.0-alpha.1`（GitHub Release），尚未上架 pub.dev 与 crates.io，先用 git 或路径依赖。

## 安装

```{tab-set}

:::{tab-item} Dart
:sync: dart

`pubspec.yaml`：

~~~yaml
dependencies:
  quanttide_founder:
    git:
      url: https://github.com/quanttide-founder/quanttide-founder-toolkit.git
      ref: dart/v0.1.0-alpha.1
      path: packages/dart
~~~

~~~sh
dart pub get
~~~

:::

:::{tab-item} Rust
:sync: rust

`Cargo.toml`（仓库根没有工作区清单，git 依赖定位不到包，先用路径依赖）：

~~~toml
[dependencies]
quanttide-founder = { path = "path/to/quanttide-founder-toolkit/packages/rust" }
~~~

上架 crates.io 后改为版本依赖：

~~~toml
[dependencies]
quanttide-founder = "0.1.0-alpha.1"
~~~

:::
```

## 跑通第一个解析

克隆主仓库后直接跑示例，示例默认定位 `assets/memory`：

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~sh
cd packages/dart
dart pub get
dart run example/parse_memory.dart
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~sh
cd packages/rust
cargo run --example parse_memory
~~~

:::
```

输出是解析报告：每个记忆集的日志时间线、档案、洞察、路线图，末尾给合计。再跑 `parse_fiction` 看小说侧：各阶段章节、编号与预留空号、观察站素材。

## 下一步

- 想在自己的仓库上用：[解析仓库](parsing.md)
- 想改「怎么读」的规则：[规则与工作流](rules.md)
- 想跑分类、分级、提炼这些计算：[编排与状态流](orchestration.md)
