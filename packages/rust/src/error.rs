//! error：横切错误类型。
//!
//! 仓库装载、定义加载、引擎编排共用一个错误值；判据是「这东西一句话是什么」。

use std::fmt;

use quanttide_agent::LLMError;

/// 工具箱统一错误。
#[derive(Debug)]
pub enum Error {
    /// 仓库根目录不存在。
    RootNotFound { domain: &'static str, path: String },
    /// 文件读写失败。
    Io(std::io::Error),
    /// 定义（YAML / JSON）或数据解析失败。
    Parse(String),
    /// workflow 里出现引擎不认识的动词。
    UnknownVerb(String),
    /// LLM 调用失败。
    Llm(LLMError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::RootNotFound { domain, path } => {
                write!(f, "{domain} 仓库根目录不存在: {path}")
            }
            Error::Io(e) => write!(f, "{e}"),
            Error::Parse(msg) => write!(f, "{msg}"),
            Error::UnknownVerb(verb) => {
                write!(f, "未知动词: {verb}（只认 scan / judge / merge）")
            }
            Error::Llm(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Llm(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Self {
        Error::Parse(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Parse(e.to_string())
    }
}

impl From<LLMError> for Error {
    fn from(e: LLMError) -> Self {
        Error::Llm(e)
    }
}
