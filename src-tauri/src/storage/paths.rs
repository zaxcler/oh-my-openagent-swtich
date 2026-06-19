//! 三平台路径解析
//!
//! - macOS: `~/Library/Application Support/oh-my-openagent-switch/`
//! - Linux: `$XDG_CONFIG_HOME/oh-my-openagent-switch/` 或 `~/.config/oh-my-openagent-switch/`
//! - Windows: `%APPDATA%/oh-my-openagent-switch/`
//!
//! opencode 配置目录：
//! - macOS/Linux: `~/.config/opencode/`
//! - Windows: `%APPDATA%/opencode/`

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
    if let Ok(env_path) = std::env::var("OMO_TEST_OPENCODE_DIR") {
        if !env_path.is_empty() {
            return Ok(PathBuf::from(env_path));
        }
    }
    dirs::config_dir()
        .map(|p| p.join("opencode"))
        .ok_or(AppError::OpencodeNotFound)
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

        assert!(configs.ends_with("oh-my-openagent-switch/configs"));
        assert!(backups.ends_with("oh-my-openagent-switch/backups"));
        assert!(opencode.ends_with("opencode"));
        assert!(active.file_name().unwrap() == "active.json");
        assert!(active.parent().unwrap().ends_with("oh-my-openagent-switch"));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_paths_linux() {
        let configs = configs_dir().unwrap();
        let backups = backups_dir().unwrap();
        let opencode = opencode_dir().unwrap();
        let active = active_file().unwrap();

        assert!(configs.ends_with("oh-my-openagent-switch/configs"));
        assert!(backups.ends_with("oh-my-openagent-switch/backups"));
        assert!(opencode.ends_with("opencode"));
        assert!(active.file_name().unwrap() == "active.json");
        assert!(active.parent().unwrap().ends_with("oh-my-openagent-switch"));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_paths_windows() {
        let configs = configs_dir().unwrap();
        let backups = backups_dir().unwrap();
        let opencode = opencode_dir().unwrap();
        let active = active_file().unwrap();

        assert!(configs.ends_with(r"oh-my-openagent-switch\configs"));
        assert!(backups.ends_with(r"oh-my-openagent-switch\backups"));
        assert!(opencode.ends_with("opencode"));
        assert!(active.file_name().unwrap() == "active.json");
        assert!(active.parent().unwrap().ends_with("oh-my-openagent-switch"));
    }
}
