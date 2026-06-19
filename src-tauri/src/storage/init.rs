//! 目录初始化

use std::fs;
use std::io;

use crate::error::AppError;
use crate::storage::paths::{backups_dir, configs_dir};

/// 递归创建 configs/ 和 backups/ 目录
///
/// 幂等：目录已存在不会报错
pub fn ensure_dirs() -> Result<(), AppError> {
    let dirs_to_create = [configs_dir()?, backups_dir()?];

    for dir in dirs_to_create {
        if dir.exists() {
            continue;
        }
        fs::create_dir_all(&dir).map_err(|e| {
            if e.kind() == io::ErrorKind::PermissionDenied {
                AppError::PermissionDenied {
                    path: dir.display().to_string(),
                }
            } else {
                AppError::IoError {
                    message: format!("创建目录失败 {}: {}", dir.display(), e),
                }
            }
        })?;
    }

    Ok(())
}
