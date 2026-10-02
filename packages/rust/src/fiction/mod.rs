//! fiction 域：观察站（素材）→ 阶段（灵感/场景/初稿/改稿/定稿）→ 成稿。
//!
//! - [`models`]：域模型（Novel / Stage / Chapter / Observation，纯数据）
//! - [`repository`]：仓库装载——发现小说与观察站、逐层读文件
//! - [`states`]：状态机（statig）承载三步提炼，结构判断收成纯函数
//! - [`rules`]：内置规则资产（共享 YAML 嵌入）——判据与提取字段都是数据

pub mod models;
pub mod repository;
pub mod rules;
pub mod states;
