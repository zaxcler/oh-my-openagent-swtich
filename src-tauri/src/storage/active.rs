//! 激活状态记录模块
//!
//! 管理 `active.json` 文件，记录当前激活的配置及其指纹。

use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::storage::paths::active_file;

/// 激活记录
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActiveRecord {
    /// 激活的配置 ID
    pub config_id: String,
    /// 激活时间
    pub applied_at: DateTime<Utc>,
    /// 配置指纹
    pub fingerprints: Fingerprints,
}

/// 配置文件指纹集
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Fingerprints {
    /// opencode.jsonc 内容指纹
    pub opencode: String,
    /// oh-my-openagent.json 内容指纹
    pub omos: String,
}

thread_local! {
    #[cfg(test)]
    pub static TEST_ACTIVE_PATH: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

fn effective_active_file() -> Result<PathBuf, AppError> {
    #[cfg(test)]
    {
        let override_path = TEST_ACTIVE_PATH.with(|cell| (*cell.borrow()).clone());
        if let Some(path) = override_path {
            return Ok(path);
        }
    }
    active_file()
}

/// 读取激活记录
///
/// - 文件不存在 → `Ok(None)`
/// - 文件损坏或格式错误 → `Err(AppError)`
pub fn read_active() -> Result<Option<ActiveRecord>, AppError> {
    let path = effective_active_file()?;
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)?;
    let record: ActiveRecord = serde_json::from_str(&content)?;
    Ok(Some(record))
}

/// 写入激活记录（原子写入：tempfile + rename）
pub fn write_active(record: &ActiveRecord) -> Result<(), AppError> {
    let path = effective_active_file()?;
    let content = serde_json::to_string_pretty(record)?;

    // 原子写入：先写临时文件再 rename
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, &content)?;
    fs::rename(&temp_path, &path)?;
    Ok(())
}

/// 删除激活记录文件
pub fn clear_active() -> Result<(), AppError> {
    let path = effective_active_file()?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_temp_path<F>(f: F)
    where
        F: FnOnce(PathBuf) + 'static,
    {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let path = tmp_dir.path().join("active.json");
        TEST_ACTIVE_PATH.with(|cell| {
            *cell.borrow_mut() = Some(path.clone());
        });
        f(path.clone());
        TEST_ACTIVE_PATH.with(|cell| {
            *cell.borrow_mut() = None;
        });
        // tmp_dir is dropped here but it must live past f()
        std::mem::forget(tmp_dir);
    }

    #[test]
    fn test_read_active_missing() {
        with_temp_path(|_path| {
            let result = read_active().unwrap();
            assert_eq!(result, None);
        });
    }

    #[test]
    fn test_write_and_read_roundtrip() {
        with_temp_path(|_path| {
            let record = ActiveRecord {
                config_id: "test-uuid-123".to_string(),
                applied_at: Utc::now(),
                fingerprints: Fingerprints {
                    opencode: "abc123".to_string(),
                    omos: "def456".to_string(),
                },
            };

            write_active(&record).unwrap();
            let loaded = read_active().unwrap();
            assert!(loaded.is_some());
            let loaded = loaded.unwrap();
            assert_eq!(loaded.config_id, "test-uuid-123");
            assert_eq!(loaded.fingerprints.opencode, "abc123");
            assert_eq!(loaded.fingerprints.omos, "def456");
        });
    }
}
