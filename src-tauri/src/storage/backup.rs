//! 备份管理
//!
//! - `backup_file`: 复制源文件到 `backups/{stem}-{timestamp}.json`，
//   同步写一个 `.original_path` 侧车文件记录原路径。
// - `list_backups`: 扫描 `backups/`，按 `created_at` 降序。
// - `restore_backup`: 从侧车读取原路径，原子写回。
// - `prune_old_backups`: 保留最近 `keep` 个，删除更早的。

#[allow(unused_imports)]
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::storage::atomic::atomic_write_json;
use crate::storage::paths::backups_dir;

thread_local! {
    #[cfg(test)]
    pub(crate) static TEST_BACKUPS_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

fn effective_backups_dir() -> Result<PathBuf, AppError> {
    #[cfg(test)]
    {
        let override_path = TEST_BACKUPS_DIR.with(|cell| (*cell.borrow()).clone());
        if let Some(path) = override_path {
            return Ok(path);
        }
    }
    backups_dir()
}

fn ensure_backups_dir() -> Result<PathBuf, AppError> {
    let dir = effective_backups_dir()?;
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// 备份元信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupMeta {
    /// 备份文件名（不含父目录）
    pub filename: String,
    /// 被备份的原始文件完整路径
    pub original_path: String,
    /// 备份创建时间
    pub created_at: DateTime<Utc>,
    /// 备份文件字节数
    pub size_bytes: u64,
}
/// Vec<BackupMeta> type alias
pub type BackupMetaVec = Vec<BackupMeta>;

fn sidecar_path_for(backup_path: &Path) -> PathBuf {
    let mut name = backup_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "backup".to_string());
    name.push_str(".original_path");
    backup_path.with_file_name(name)
}

fn write_sidecar(backup_path: &Path, original_path: &Path) -> Result<(), AppError> {
    let sidecar = sidecar_path_for(backup_path);
    fs::write(&sidecar, original_path.to_string_lossy().as_bytes())?;
    Ok(())
}

fn read_sidecar(backup_path: &Path) -> Result<Option<String>, AppError> {
    let sidecar = sidecar_path_for(backup_path);
    if !sidecar.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&sidecar)?;
    Ok(Some(content))
}

/// 备份源文件
///
/// 流程：
/// 1. 确保 `backups_dir` 存在
/// 2. 读取源文件内容
/// 3. 写到 `backups/{file_stem}-{timestamp}.json`
/// 4. 写侧车 `.original_path` 记录原路径
/// 5. 返回备份文件 PathBuf
pub fn backup_file(path: &Path) -> Result<PathBuf, AppError> {
    if !path.exists() {
        return Err(AppError::FileNotFound {
            path: path.to_string_lossy().to_string(),
        });
    }

    let content = fs::read_to_string(path).map_err(|e| AppError::IoError {
        message: format!("读取源文件失败 {}: {}", path.display(), e),
    })?;

    let dir = ensure_backups_dir()?;
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| AppError::BackupFailed {
            reason: format!("无法提取文件 stem: {}", path.display()),
        })?;
    let timestamp = Utc::now().format("%Y-%m-%dT%H-%M-%S-%3f").to_string();
    let filename = format!("{}-{}.json", stem, timestamp);
    let backup_path = dir.join(&filename);

    fs::write(&backup_path, &content).map_err(|e| AppError::BackupFailed {
        reason: format!("写入备份失败 {}: {}", backup_path.display(), e),
    })?;
    write_sidecar(&backup_path, path)?;

    Ok(backup_path)
}

/// 列出所有备份，按 `created_at` 降序
pub fn list_backups() -> Result<BackupMetaVec, AppError> {

    let dir = effective_backups_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut metas = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let original_path = match read_sidecar(&path)? {
            Some(p) => p,
            None => continue,
        };

        let metadata = fs::metadata(&path)?;
        let created_at: DateTime<Utc> = metadata
            .created()
            .or_else(|_| metadata.modified())
            .unwrap_or_else(|_| std::time::SystemTime::now())
            .into();

        metas.push(BackupMeta {
            filename,
            original_path,
            created_at,
            size_bytes: metadata.len(),
        });
    }

    metas.sort_by_key(|m| std::cmp::Reverse(m.created_at));
    Ok(metas)
}

/// 从备份还原到原路径
///
/// 读取侧车找到原路径，解析备份内容为 JSON 后原子写回。
pub fn restore_backup(backup_filename: &str) -> Result<(), AppError> {
    let dir = effective_backups_dir()?;
    let backup_path = dir.join(backup_filename);
    if !backup_path.exists() {
        return Err(AppError::FileNotFound {
            path: backup_path.to_string_lossy().to_string(),
        });
    }

    let original_path = read_sidecar(&backup_path)?.ok_or_else(|| AppError::BackupFailed {
        reason: format!("备份缺少原始路径元数据: {}", backup_path.display()),
    })?;

    let content = fs::read_to_string(&backup_path).map_err(|e| AppError::IoError {
        message: format!("读取备份失败 {}: {}", backup_path.display(), e),
    })?;

    let value: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| AppError::BackupFailed {
            reason: format!("备份内容不是合法 JSON: {}", e),
        })?;

    let target = PathBuf::from(&original_path);
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    atomic_write_json(&target, &value)
}

/// 删除指定备份文件（含 `.original_path` 侧车文件）
///
/// - 文件不存在 → `FileNotFound`
/// - 删除主文件后顺手清理侧车
pub fn delete_backup(backup_filename: &str) -> Result<(), AppError> {
    let dir = effective_backups_dir()?;
    let backup_path = dir.join(backup_filename);
    if !backup_path.exists() {
        return Err(AppError::FileNotFound {
            path: backup_path.to_string_lossy().to_string(),
        });
    }

    fs::remove_file(&backup_path).map_err(|e| AppError::IoError {
        message: format!("删除备份失败 {}: {}", backup_path.display(), e),
    })?;

    let sidecar = sidecar_path_for(&backup_path);
    if sidecar.exists() {
        // 侧车删除失败不视为致命错误（主文件已删）
        let _ = fs::remove_file(&sidecar);
    }

    Ok(())
}

/// 保留最近 `keep` 个备份，删除更早的
pub fn prune_old_backups(keep: usize) -> Result<(), AppError> {
    let mut backups = list_backups()?;
    if backups.len() <= keep {
        return Ok(());
    }

    let dir = effective_backups_dir()?;
    for meta in backups.drain(keep..) {
        let backup_path = dir.join(&meta.filename);
        if backup_path.exists() {
            fs::remove_file(&backup_path)?;
        }
        let sidecar = sidecar_path_for(&backup_path);
        if sidecar.exists() {
            fs::remove_file(&sidecar)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    fn with_backups_dir<F>(f: F)
    where
        F: FnOnce(PathBuf),
    {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().to_path_buf();
        TEST_BACKUPS_DIR.with(|cell| {
            *cell.borrow_mut() = Some(path.clone());
        });
        f(path.clone());
        TEST_BACKUPS_DIR.with(|cell| {
            *cell.borrow_mut() = None;
        });
    }

    fn write_source(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    fn sleep_one_ms() {
        thread::sleep(Duration::from_millis(2));
    }

    #[test]
    fn test_backup_roundtrip() {
        with_backups_dir(|backup_root| {
            let source_dir = TempDir::new().unwrap();
            let source_path = write_source(source_dir.path(), "opencode.jsonc", r#"{"k":"v1"}"#);

            let backup_path = backup_file(&source_path).unwrap();
            assert!(backup_path.exists());
            assert!(backup_path.starts_with(&backup_root));

            fs::write(&source_path, r#"{"k":"v2"}"#).unwrap();

            let filename = backup_path.file_name().unwrap().to_str().unwrap().to_string();
            restore_backup(&filename).unwrap();

            let restored = fs::read_to_string(&source_path).unwrap();
            let value: serde_json::Value = serde_json::from_str(&restored).unwrap();
            assert_eq!(value["k"], "v1");
        });
    }

    #[test]
    fn test_backup_timestamp_format() {
        with_backups_dir(|_dir| {
            let source_dir = TempDir::new().unwrap();
            let source_path = write_source(source_dir.path(), "opencode.jsonc", "{}");
            let backup_path = backup_file(&source_path).unwrap();
            let filename = backup_path.file_name().unwrap().to_str().unwrap().to_string();

            assert!(
                filename.contains("2026-") || filename.contains("2025-") || filename.contains("2024-"),
                "filename should contain a year prefix: {}",
                filename
            );
            let ts_part = filename
                .trim_start_matches("opencode-")
                .trim_end_matches(".json");
            assert!(
                ts_part.contains('T'),
                "timestamp should contain 'T' separator: {}",
                filename
            );
            assert!(
                ts_part.len() >= 19,
                "timestamp should be at least YYYY-MM-DDTHH-MM-SS long: {}",
                filename
            );
        });
    }

    #[test]
    fn test_list_backups_sorted() {
        with_backups_dir(|_dir| {
            let source_dir = TempDir::new().unwrap();
            let source_path = write_source(source_dir.path(), "config.json", "1");

            let b1 = backup_file(&source_path).unwrap();
            sleep_one_ms();
            let _b2 = backup_file(&source_path).unwrap();
            sleep_one_ms();
            let b3 = backup_file(&source_path).unwrap();

            let list = list_backups().unwrap();
            assert_eq!(list.len(), 3);
            assert_eq!(list[0].filename, b3.file_name().unwrap().to_str().unwrap());
            assert_eq!(list[2].filename, b1.file_name().unwrap().to_str().unwrap());
        });
    }

    #[test]
    fn test_prune_keeps_recent() {
        with_backups_dir(|_dir| {
            let source_dir = TempDir::new().unwrap();
            let source_path = write_source(source_dir.path(), "config.json", "0");

            let mut created = Vec::new();
            for i in 0..5 {
                fs::write(&source_path, format!("{}", i)).unwrap();
                let bp = backup_file(&source_path).unwrap();
                created.push(bp);
                sleep_one_ms();
            }

            assert_eq!(list_backups().unwrap().len(), 5);
            prune_old_backups(3).unwrap();
            let remaining = list_backups().unwrap();
            assert_eq!(remaining.len(), 3);

            let kept_filenames: Vec<String> = remaining.iter().map(|m| m.filename.clone()).collect();
            let expected: Vec<String> = created[2..]
                .iter()
                .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
                .collect();
            for name in &expected {
                assert!(kept_filenames.contains(name), "should keep {}", name);
            }
        });
    }

    #[test]
    fn test_restore_missing_backup() {
        with_backups_dir(|_dir| {
            let result = restore_backup("non-existent-2026-01-01T00-00-00.json");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, AppError::FileNotFound { .. }));
        });
    }

    #[test]
    fn test_delete_backup_removes_file_and_sidecar() {
        with_backups_dir(|_dir| {
            let source_dir = TempDir::new().unwrap();
            let source_path = write_source(source_dir.path(), "config.json", "x");

            let backup_path = backup_file(&source_path).unwrap();
            let filename = backup_path.file_name().unwrap().to_str().unwrap().to_string();
            let sidecar = sidecar_path_for(&backup_path);
            assert!(backup_path.exists());
            assert!(sidecar.exists());

            delete_backup(&filename).unwrap();
            assert!(!backup_path.exists());
            assert!(!sidecar.exists());
        });
    }

    #[test]
    fn test_delete_backup_missing() {
        with_backups_dir(|_dir| {
            let result = delete_backup("non-existent.json");
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), AppError::FileNotFound { .. }));
        });
    }

    #[test]
    fn test_backup_missing_source() {
        with_backups_dir(|_dir| {
            let source_dir = TempDir::new().unwrap();
            let missing = source_dir.path().join("does-not-exist.json");
            let result = backup_file(&missing);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, AppError::FileNotFound { .. }));
        });
    }
}
