//! memory/rules：内置规则资产——与 Dart 包共享的 YAML（`tests/fixtures/`），编译期嵌入。
//!
//! 类别与判据是数据：四份 [`Artifact`] 的 `name` 就是去向（journal 是它们的补集），
//! 判据写在各自的 `criteria` 里由 LLM 执行；分级工作流的判据在 `workflows/*.yaml`。
//! 换一份 YAML 就换一种解读方式，代码不动；运行期也可以 `Artifact::from_file` 载入自己的规则。

use std::sync::LazyLock;

use crate::core::rules::{Artifact, Workflow};

/// 认知层：洞察的读法与判据（含分级类别）。
pub static INSIGHT: LazyLock<Artifact> = LazyLock::new(|| {
    Artifact::from_yaml(include_str!(
        "../../../../tests/fixtures/rules/insight.yaml"
    ))
    .expect("内置 insight.yaml 合法")
});

/// 特征层：档案的读法与判据。
pub static PROFILE: LazyLock<Artifact> = LazyLock::new(|| {
    Artifact::from_yaml(include_str!(
        "../../../../tests/fixtures/rules/profile.yaml"
    ))
    .expect("内置 profile.yaml 合法")
});

/// 方向层：路线图的读法与判据。
pub static ROADMAP: LazyLock<Artifact> = LazyLock::new(|| {
    Artifact::from_yaml(include_str!(
        "../../../../tests/fixtures/rules/roadmap.yaml"
    ))
    .expect("内置 roadmap.yaml 合法")
});

/// 意愿层：意图的读法与判据。
pub static INTENTION: LazyLock<Artifact> = LazyLock::new(|| {
    Artifact::from_yaml(include_str!(
        "../../../../tests/fixtures/rules/intention.yaml"
    ))
    .expect("内置 intention.yaml 合法")
});

/// 证据分级的判据与决策表。
pub static GRADE: LazyLock<Workflow> = LazyLock::new(|| {
    Workflow::from_yaml(include_str!(
        "../../../../tests/fixtures/workflows/grade.yaml"
    ))
    .expect("内置 grade.yaml 合法")
});

/// 聚类与合并的判据、可选决策。
pub static CLUSTER: LazyLock<Workflow> = LazyLock::new(|| {
    Workflow::from_yaml(include_str!(
        "../../../../tests/fixtures/workflows/cluster.yaml"
    ))
    .expect("内置 cluster.yaml 合法")
});

/// 去向分类的在册类别：都不命中或拿不准 = 留在 journal（补集，不是类别）。
///
/// 返回克隆的一份，调用方拿到的是 `&[Artifact]`，批量分类与自定义类别同型。
pub fn categories() -> Vec<Artifact> {
    [&INSIGHT, &PROFILE, &ROADMAP, &INTENTION]
        .iter()
        .map(|artifact| (**artifact).clone())
        .collect()
}
