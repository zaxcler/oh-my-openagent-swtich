//! 导出模块
//!
//! - `export_config`: 将配置导出到目标路径

use std::path::Path;

use crate::error::AppError;
use crate::storage::atomic::atomic_write;
use crate::storage::configs::Config;

/// 导出配置到目标路径
///
/// 规则：
/// - 将 config 序列化为 JSON 写入 target 路径
/// - target 路径由调用方指定（不自动加后缀）
/// - 覆盖已存在的 target 文件（原子写入：tempfile + rename）
pub fn export_config(config: &Config, target: &Path) -> Result<(), AppError> {
    let content = serde_json::to_string_pretty(config)?;
    atomic_write(target, &content)
}
