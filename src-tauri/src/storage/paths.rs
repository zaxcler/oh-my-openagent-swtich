//! 三平台路径解析
//!
//! - macOS: `~/Library/Application Support/oh-my-openagent-switch/`
//! - Linux: `$XDG_CONFIG_HOME/oh-my-openagent-switch/` 或 `~/.config/oh-my-openagent-switch/`
//! - Windows: `%APPDATA%/oh-my-openagent-switch/`
//!
//! opencode 配置目录（按优先级解析）：
//! 1. `OPENCODE_CONFIG_DIR` 环境变量（用户显式指定目录）
//! 2. `OPENCODE_CONFIG` 环境变量（用户显式指定文件路径，提取父目录）
//! 3. 系统默认 `dirs::config_dir().join("opencode")`
//!
//! oh-my-openagent.json 候选名（按 oh-my-openagent 官方文档 File Locations）：
//! - 用户级：`~/.config/opencode/oh-my-openagent.json[c]`（或 legacy `oh-my-opencode.json[c]`）
//! - 项目级：`.opencode/oh-my-openagent.json[c]`（从 cwd 向上走到 $HOME，closer wins）

#[allow(unused_imports)]
use std::cell::RefCell;
use std::path::PathBuf;

use crate::error::AppError;

const APP_NAME: &str = "oh-my-openagent-switch";

fn app_config_root() -> Result<PathBuf, AppError> {
    dirs::config_dir()
        .map(|p| p.join(APP_NAME))
        .ok_or(AppError::OpencodeNotFound)
}

pub fn configs_dir() -> Result<PathBuf, AppError> {
    configs_dir_with_override(None)
}

pub fn configs_dir_with_override(override_path: Option<PathBuf>) -> Result<PathBuf, AppError> {
    if let Some(p) = override_path {
        return Ok(p);
    }
    if let Ok(env_path) = std::env::var("OMO_TEST_CONFIGS_DIR") {
        return Ok(PathBuf::from(env_path));
    }
    Ok(app_config_root()?.join("configs"))
}

pub fn backups_dir() -> Result<PathBuf, AppError> {
    backups_dir_with_override(None)
}

pub fn backups_dir_with_override(override_path: Option<PathBuf>) -> Result<PathBuf, AppError> {
    if let Some(p) = override_path {
        return Ok(p);
    }
    if let Ok(env_path) = std::env::var("OMO_TEST_BACKUPS_DIR") {
        return Ok(PathBuf::from(env_path));
    }
    Ok(app_config_root()?.join("backups"))
}

thread_local! {
    static TEST_OPENCODE_DIR: std::cell::RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

pub fn opencode_dir() -> Result<PathBuf, AppError> {
    {
        let override_path = TEST_OPENCODE_DIR.with(|cell| (*cell.borrow()).clone());
        if let Some(path) = override_path {
            return Ok(path);
        }
    }
    if let Ok(env_path) = std::env::var("OPENCODE_CONFIG_DIR") {
        if !env_path.is_empty() {
            return Ok(PathBuf::from(env_path));
        }
    }
    if let Ok(env_path) = std::env::var("OPENCODE_CONFIG") {
        if !env_path.is_empty() {
            let p = PathBuf::from(env_path);
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    return Ok(parent.to_path_buf());
                }
            }
        }
    }
    // macOS 上 dirs::config_dir() 返回 ~/Library/Application Support，
    // 但 opencode 配置实际在 ~/.config/opencode（遵循 XDG 规范）。
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let xdg_config = PathBuf::from(home).join(".config").join("opencode");
            return Ok(xdg_config);
        }
    }
    dirs::config_dir()
        .map(|p| p.join("opencode"))
        .ok_or(AppError::OpencodeNotFound)
}

/// oh-my-openagent.json 候选文件名（兼容 rename transition）
pub fn omos_candidate_names() -> &'static [&'static str] {
    &["oh-my-openagent.json", "oh-my-opencode.json"]
}

/// 找到实际存在的 omos 配置文件（按 oh-my-openagent 官方文档顺序：先 legacy）
/// 若都不存在返回 None
pub fn existing_omos_path() -> Result<Option<PathBuf>, AppError> {
    let dir = opencode_dir()?;
    for name in omos_candidate_names() {
        let p = dir.join(name);
        if p.exists() {
            return Ok(Some(p));
        }
    }
    Ok(None)
}

/// 默认 omos 写入路径（用户级），新文件用新名 `oh-my-openagent.json`
pub fn omos_path() -> Result<PathBuf, AppError> {
    Ok(opencode_dir()?.join("oh-my-openagent.json"))
}

pub fn set_test_opencode_dir(path: PathBuf) {
    TEST_OPENCODE_DIR.with(|cell| *cell.borrow_mut() = Some(path));
}

pub fn clear_test_opencode_dir() {
    TEST_OPENCODE_DIR.with(|cell| *cell.borrow_mut() = None);
}

pub fn active_file() -> Result<PathBuf, AppError> {
    Ok(app_config_root()?.join("active.json"))
}

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_paths_macos() {
        let configs = configs_dir().unwrap();
        let backups = backups_dir().unwrap();
        let opencode = opencode_dir().unwrap();
        let active = active_file().unwrap();
        let omos = omos_path().unwrap();

        assert!(configs.ends_with("oh-my-openagent-switch/configs"));
        assert!(backups.ends_with("oh-my-openagent-switch/backups"));
        // macOS 上 opencode 配置在 ~/.config/opencode，而非 ~/Library/Application Support/opencode
        assert!(
            opencode.ends_with(".config/opencode"),
            "opencode_dir 应返回 ~/.config/opencode，实际: {}",
            opencode.display()
        );
        assert!(active.file_name().unwrap() == "active.json");
        assert!(active.parent().unwrap().ends_with("oh-my-openagent-switch"));
        assert!(omos.file_name().unwrap() == "oh-my-openagent.json");
        assert!(
            omos.parent().unwrap().ends_with(".config/opencode"),
            "omos_path 父目录应为 ~/.config/opencode，实际: {}",
            omos.parent().unwrap().display()
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_paths_linux() {
        let configs = configs_dir().unwrap();
        let backups = backups_dir().unwrap();
        let opencode = opencode_dir().unwrap();
        let active = active_file().unwrap();
        let omos = omos_path().unwrap();

        assert!(configs.ends_with("oh-my-openagent-switch/configs"));
        assert!(backups.ends_with("oh-my-openagent-switch/backups"));
        assert!(opencode.ends_with("opencode"));
        assert!(active.file_name().unwrap() == "active.json");
        assert!(active.parent().unwrap().ends_with("oh-my-openagent-switch"));
        assert!(omos.file_name().unwrap() == "oh-my-openagent.json");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_paths_windows() {
        let configs = configs_dir().unwrap();
        let backups = backups_dir().unwrap();
        let opencode = opencode_dir().unwrap();
        let active = active_file().unwrap();
        let omos = omos_path().unwrap();

        assert!(configs.ends_with(r"oh-my-openagent-switch\configs"));
        assert!(backups.ends_with(r"oh-my-openagent-switch\backups"));
        assert!(opencode.ends_with("opencode"));
        assert!(active.file_name().unwrap() == "active.json");
        assert!(active.parent().unwrap().ends_with("oh-my-openagent-switch"));
        assert!(omos.file_name().unwrap() == "oh-my-openagent.json");
    }
}
