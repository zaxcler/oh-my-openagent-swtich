//! 原子写入（tempfile + rename）
//!
//! POSIX 上 `rename` 原子替换同卷文件，避免半写状态。Windows 上
//! `std::fs::rename` 在同目录场景下也表现为原子替换。

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::error::AppError;

/// 同目录下的临时文件路径
///
/// 形如 `.{file_name}.tmp`，与目标在同一目录，保证 `rename` 在同卷上执行。
fn temp_path_for(path: &Path) -> std::path::PathBuf {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "atomic".to_string());
    path.with_file_name(format!(".{}.tmp", file_name))
}

/// 原子写入字符串内容到 `path`
///
/// 先写到 `.tmp` 临时文件，再 `rename` 替换。如果中间任何一步失败，
/// 目标文件保持原样不变（要么旧、要么新，不会半写）。
pub fn atomic_write(path: &Path, content: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let temp_path = temp_path_for(path);
    fs::write(&temp_path, content)?;

    match fs::rename(&temp_path, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&temp_path);
            Err(e.into())
        }
    }
}

/// 原子写入 `serde_json::Value` 到 `path`
///
/// 先 `serde_json::to_string_pretty` 序列化，再调 `atomic_write`。
pub fn atomic_write_json(path: &Path, value: &Value) -> Result<(), AppError> {
    let content = serde_json::to_string_pretty(value)?;
    atomic_write(path, &content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn test_atomic_write_json() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("data.json");
        let value = json!({
            "name": "omos",
            "agents": { "coder": "gpt-4o" },
            "categories": { "web": "claude-3" }
        });
        atomic_write_json(&path, &value).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        let parsed: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed, value);
    }

    #[test]
    fn test_atomic_write_no_half_file() {
        let tmp = TempDir::new().unwrap();
        let block_dir = tmp.path().join("block");
        fs::create_dir(&block_dir).unwrap();
        let target = block_dir.join("file.json");
        fs::write(&target, "original").unwrap();

        // 把 block_dir 设为只读，强制 fs::write(tmp) 失败
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o555);
            fs::set_permissions(&block_dir, perms).unwrap();
        }
        #[cfg(not(unix))]
        {
            let mut perms = fs::metadata(&block_dir).unwrap().permissions();
            perms.set_readonly(true);
            fs::set_permissions(&block_dir, perms).unwrap();
        }

        let result = atomic_write(&target, "new-content");

        // 恢复权限以便清理
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            fs::set_permissions(&block_dir, perms).unwrap();
        }
        #[cfg(not(unix))]
        {
            let mut perms = fs::metadata(&block_dir).unwrap().permissions();
            perms.set_readonly(false);
            fs::set_permissions(&block_dir, perms).unwrap();
        }

        // 关键断言：原文件保持不变
        assert!(result.is_err(), "应该返回错误（tmp 无法写入）");
        let still = fs::read_to_string(&target).unwrap();
        assert_eq!(still, "original", "原文件必须保持不变");
    }
}
