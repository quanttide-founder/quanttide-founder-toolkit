//! memory 域：时间线（日志）→ 特征（档案）→ 认知（洞察）→ 方向（路线图）。
//!
//! - [`models`]：域模型（纯数据，不读文件）
//! - [`repository`]：仓库装载——发现记忆集、逐层读文件
//! - [`states`]：状态机（statig）承载四步流程，判断规则收成纯函数
//! - [`rules`]：内置规则资产（共享 YAML 嵌入）——类别、判据、分级都是数据

pub mod models;
pub mod repository;
pub mod rules;
pub mod states;
