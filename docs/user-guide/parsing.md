# 解析仓库

装载是唯一的入口：给一个仓库根目录，按目录命名约定发现内容，逐层读成模型。两仓库的共同结构（特征目录、文件名元数据、允许结构缺失）见[工具箱总述](../index.md#两个仓库的共同结构)，这里只讲怎么用。

## 装载

根目录下含任一特征子目录的目录算一个记忆集；含 `index.md` 或 `N_` 阶段子目录的目录算一部小说。根目录不存在时报错，隐藏目录跳过。

```{tab-set}

:::{tab-item} Dart
:sync: dart

~~~dart
final memory = MemoryRepository.load('/path/to/memory');
for (final set in memory.sets) {
  print('${set.name}: ${set.journals.length} 篇日志');
}

final fiction = FictionRepository.load('/path/to/fiction');
for (final novel in fiction.novels) {
  print('${novel.name}: ${novel.stages.length} 个阶段');
}
~~~

:::

:::{tab-item} Rust
:sync: rust

~~~rust
let memory = MemoryRepository::load("/path/to/memory")?;
for set in &memory.sets {
    println!("{}: {} 篇日志", set.name, set.journals.len());
}

let fiction = FictionRepository::load("/path/to/fiction")?;
for novel in &fiction.novels {
    println!("{}: {} 个阶段", novel.name, novel.stages.len());
}
~~~

:::
```

## 记忆集的四层

装载后的 `MemorySet` 带四份产出，各有各的读法：

- `journals`：日志时间线，文件名即日期，按日期倒序；`---` 分隔线切成会话段，集根放当天、`journal/` 放归档
- `profiles`：档案，首个 H1 作标题、H1 与首个 H2 之间的段落作说明、按 H2 切主题节
- `insights`：洞察，按 H2 分「已确认 / 假说」两档，条目拆成命题、解释、证据三段（证据取「依据：」之后）
- `roadmaps`：路线图，识别目标、元目标、核心问题、已决策、待决策；认不出的节保留为主题节

结构缺失不报错：没有 H2 的洞察按主题式散文读，路线图缺目标、档案缺一级标题都照常解析。

## 小说与观察站

`FictionRepository` 装载两类东西：

- `novels`：一部小说若干 `Stage`（目录名 `{N}_{名称}` 动态识别，不硬编码阶段名）与 `Chapter`（`{序号}_{标题}.md`）；`0_` 前缀是前言，不占正文章节号，未编号文件是替代草稿
- `observation`：观察站的情绪日记与社会观察，素材的来源，不含 AI 产物（`实验室/` 由使用方单独处理）

编号轴的语义——各阶段共用同一编号轴、预留空号宁空勿移、同号多份即同一章的不同版本——见[编排与状态流](orchestration.md#章节编号轴)。

## 读字段

模型字段与两语言的字段清单见 [API 参考](../api-references/index.md)，常用入口是 `JournalEntry.segments()`（会话段）与 `JournalEntry.characterCount`（去空白字数）。仓库只读文件、不做语义；语义在解析与规则层。
