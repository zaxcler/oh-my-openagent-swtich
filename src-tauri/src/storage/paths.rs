//! 三平台路径解析
//!
//! - macOS: `~/Library/Application Support/oh-my-openagent-switch/`
//! - Linux: `$XDG_CONFIG_HOME/oh-my-openagent-switch/` 或 `~/.config/oh-my-openagent-switch/`
//! - Windows: `%APPDATA%/oh-my-openagent-switch/`
//!
//! opencode 配置目录：
//! - macOS/Linux: `~/.config/opencode/`
//! - Windows: `%APPDATA%/opencode/`

use std::path::PathBuf;

use crate::error::AppError;

const APP_NAME: &str = "oh-my-openagent-switch";

/// 获取应用配置目录路径
fn app_config_root() -> Result<PathBuf, AppError> {
    dirs::config_dir()
        .map(|p| p.join(APP_NAME))
        .ok_or(AppError::OpencodeNotFound)
}

/// 获取 configs 目录路径
///
/// - macOS: `~/Library/Application Support/oh-my-openagent-switch/configs/`
/// - Linux: `~/.config/oh-my-openagent-switch/configs/`
/// - Windows: `%APPDATA%\oh-my-openagent-switch\configs\`
pub fn configs_dir() -> Result<PathBuf, AppError> {
    Ok(app_config_root()?.join("configs"))
}

/// 获取 backups 目录路径
///
/// - macOS: `~/Library/Application Support/oh-my-openagent-switch/backups/`
/// - Linux: `~/.config/oh-my-openagent-switch/backups/`
/// - Windows: `%APPDATA%\oh-my-openagent-switch\backups\`
pub fn backups_dir() -> Result<PathBuf, AppError> {
    Ok(app_config_root()?.join("backups"))
}

/// 获取 opencode 配置目录路径
///
/// - macOS/Linux: `~/.config/opencode/`
/// - Windows: `%APPDATA%\opencode\`
pub fn opencode_dir() -> Result<PathBuf, AppError> {
    dirs::config_dir()
        .map(|p| p.join("opencode"))
        .ok_or(AppError::OpencodeNotFound)
}

/// 获取活动配置文件路径
///
/// - macOS: `~/Library/Application Support/oh-my-openagent-switch/active.json`
/// - Linux: `~/.config/oh-my-openagent-switch/active.json`
/// - Windows: `%APPDATA%\oh-my-openagent-switch\active.json`
pub fn active_file() -> Result<PathBuf, AppError> {
    Ok(app_config_root()?.join("active.json"))
}

#[cfg(test)]
mod tests {
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
        assert!(opencode.ends_with(r"opencode"));
        assert!(active.file_name().unwrap() == "active.json");
        assert!(active.parent().unwrap().ends_with("oh-my-openagent-switch"));
    }
}
