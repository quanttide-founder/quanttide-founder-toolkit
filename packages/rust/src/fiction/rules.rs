//! fiction/rules：内置规则资产——与 Dart 包共享的 YAML（`tests/fixtures/`），编译期嵌入。
//!
//! 取样、观察展开、落实片段、包装文案的判据都在 `criteria` 里，提取字段在 `sections.extract` 里；
//! 换一份 YAML 就换一种提炼方式，代码不动。

use std::sync::LazyLock;

use crate::core::rules::Artifact;

/// 观察素材的判据与读法（取样、观察展开）。
pub static OBSERVATION: LazyLock<Artifact> = LazyLock::new(|| {
    Artifact::from_yaml(include_str!(
        "../../../../tests/fixtures/rules/observation.yaml"
    ))
    .expect("内置 observation.yaml 合法")
});

/// 创作片段的提取规则（母题 + 场景）。
pub static FRAGMENT: LazyLock<Artifact> = LazyLock::new(|| {
    Artifact::from_yaml(include_str!(
        "../../../../tests/fixtures/rules/fragment.yaml"
    ))
    .expect("内置 fragment.yaml 合法")
});

/// 包装文案的提取规则（标题 / 简介 / 立意）。
pub static PACKAGING: LazyLock<Artifact> = LazyLock::new(|| {
    Artifact::from_yaml(include_str!(
        "../../../../tests/fixtures/rules/packaging.yaml"
    ))
    .expect("内置 packaging.yaml 合法")
});
