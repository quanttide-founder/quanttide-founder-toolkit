//! 量潮创始人工具箱（Rust 包）。
//!
//! 人机交互框架的 Rust 实现：人往里写（日志、小说、洞察），AI 往外用（提炼、建模、续写）。
//! 通用的读写基础是三步——语法解析（[`core::parse`]）、结构切分（[`core::parse::split_sections`]）、
//! 语义提取（[`core::engine`]）；memory 与 fiction 两个域共用这套解析，只换规则文件。
//!
//! 与 Dart 包 `quanttide_founder` 行为对齐：LLM 调用不自带客户端，用 `quanttide-agent` 的 `LLM`；
//! 域编排不用手写状态流转，用 `statig` 状态机（[`memory::states`] / [`fiction::states`]）。

pub mod core;
pub mod error;
pub mod fiction;
pub mod memory;

/// 状态机库再导出——驱动 [`memory::states`] / [`fiction::states`] 里的流程时，
/// 用 `quanttide_founder::statig::prelude::*` 即可，不必另加依赖。
pub use statig;

pub use crate::core::engine::{
    Candidate, Engine, ExtractionResult, Judgment, LlmExtractor, RuleBasedExtractor,
    SemanticExtractor, WorkflowResult,
};
pub use crate::core::parse::{
    Block, BlockType, MarkdownDocument, NamedItem, RawSection, TextSection, parse_named_item,
    split_sections,
};
pub use crate::core::rules::{
    Action, Artifact, Criteria, DescriptionLocation, GradeRule, SplitLevel, Step, TitleSource,
    Verb, Workflow,
};
pub use crate::error::Error;
pub use crate::fiction::models::{
    Chapter, EmotionalDiary, Novel, Observation, SocialObservation, Stage,
};
pub use crate::fiction::repository::FictionRepository;
pub use crate::fiction::states::{
    Event as FictionEvent, ExtractedFragment, FictionFlow, FictionState, NumberAssignment,
    Packaging, StageStatus,
};
pub use crate::memory::models::{
    InsightDoc, InsightGrade, InsightItem, InsightSection, JournalEntry, JournalSource, ProfileDoc,
    RoadmapDoc, Tag,
};
pub use crate::memory::repository::{MemoryRepository, MemorySet};
pub use crate::memory::states::{
    ClassifiedEntry, Event as MemoryEvent, GradedInsight, MemoryFlow, MemoryState, MergeAction,
    MergeDecision,
};
