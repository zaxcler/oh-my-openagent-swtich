//! 统一错误类型定义

use serde::Serialize;
use std::fmt;
use std::io;
use std::string::FromUtf8Error;

/// 应用程序统一错误类型，覆盖所有关键场景
#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "details")]
pub enum AppError {
    /// 文件未找到
    FileNotFound { path: String },
    /// 权限不足
    PermissionDenied { path: String },
    /// JSON 解析失败
    InvalidJson { path: String },
    /// 备份操作失败
    BackupFailed { reason: String },
    /// 活动配置文件缺失
    ActiveConfigMissing,
    /// 指定配置不存在
    ConfigNotFound { name: String },
    /// IO 错误（底层）
    IoError { message: String },
    /// JSONC 解析错误
    JsoncParse { path: String },
    /// opencode 安装目录未找到
    OpencodeNotFound,
    /// provider 不匹配
    ProviderMismatch { expected: String, found: String },
    /// 未知错误
    Unknown { message: String },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::FileNotFound { path } => {
                write!(f, "文件未找到: {}", path)
            }
            AppError::PermissionDenied { path } => {
                write!(f, "权限不足，无法访问: {}", path)
            }
            AppError::InvalidJson { path } => {
                write!(f, "JSON 解析失败: {}", path)
            }
            AppError::BackupFailed { reason } => {
                write!(f, "备份失败: {}", reason)
            }
            AppError::ActiveConfigMissing => {
                write!(f, "活动配置文件缺失（active.json 未找到或无效）")
            }
            AppError::ConfigNotFound { name } => {
                write!(f, "配置不存在: {}", name)
            }
            AppError::IoError { message } => {
                write!(f, "IO 错误: {}", message)
            }
            AppError::JsoncParse { path } => {
                write!(f, "JSONC 解析错误: {}", path)
            }
            AppError::OpencodeNotFound => {
                write!(f, "opencode 配置目录未找到，请确认已安装 opencode")
            }
            AppError::ProviderMismatch { expected, found } => {
                write!(f, "provider 不匹配，期望 {}，实际 {}", expected, found)
            }
            AppError::Unknown { message } => {
                write!(f, "未知错误: {}", message)
            }
        }
    }
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => AppError::IoError {
                message: format!("文件或目录不存在: {}", err),
            },
            io::ErrorKind::PermissionDenied => AppError::IoError {
                message: format!("权限不足: {}", err),
            },
            _ => AppError::IoError {
                message: err.to_string(),
            },
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::InvalidJson {
            path: err.to_string(),
        }
    }
}

impl From<jsonc_parser::errors::ParseError> for AppError {
    fn from(err: jsonc_parser::errors::ParseError) -> Self {
        AppError::JsoncParse {
            path: err.to_string(),
        }
    }
}

impl From<FromUtf8Error> for AppError {
    fn from(err: FromUtf8Error) -> Self {
        AppError::Unknown {
            message: format!("UTF-8 编码错误: {}", err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_file_not_found() {
        let err = AppError::FileNotFound {
            path: "/tmp/test.json".to_string(),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains(r#""kind":"FileNotFound""#));
        assert!(json.contains(r#""path":"/tmp/test.json""#));
    }

    #[test]
    fn test_serialize_unknown() {
        let err = AppError::Unknown {
            message: "something went wrong".to_string(),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains(r#""kind":"Unknown""#));
        assert!(json.contains(r#""message":"something went wrong""#));
    }
}
