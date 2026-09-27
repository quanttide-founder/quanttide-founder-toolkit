//! core：跨域共享（域依赖 core，core 不依赖域）。
//!
//! - [`parse`]：Markdown 解析 + 结构切分
//! - [`rules`]：Artifact / Workflow 加载
//! - [`engine`]：语义提取 + scan / judge / merge
//!
//! LLM 调用接口不在此定义——直接用 `quanttide-agent` 的 `LLM`。

pub mod engine;
pub mod parse;
pub mod rules;
