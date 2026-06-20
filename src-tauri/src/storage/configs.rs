use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::storage::paths::configs_dir;

thread_local! {
    pub(crate) static TEST_CONFIGS_DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

fn effective_configs_dir() -> Result<PathBuf, AppError> {
    #[cfg(test)]
    {
        let override_path = TEST_CONFIGS_DIR.with(|cell| (*cell.borrow()).clone());
        if let Some(path) = override_path {
            return Ok(path);
        }
    }
    configs_dir()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub id: String,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub payload: ConfigPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPayload {
    pub label: String,
    pub provider: ConfigProvider,
    pub agents: HashMap<String, String>,
    pub categories: HashMap<String, String>,
    /// 自动导入时记录源文件路径，用于去重
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigProvider {
    pub name: String,
    pub npm: String,
    pub options: ProviderOptions,
    pub models: HashMap<String, ModelEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderOptions {
    pub api_key: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub name: String,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMeta {
    pub id: String,
    pub label: String,
    pub updated_at: DateTime<Utc>,
}

fn config_path(id: &str, dir: &Path) -> PathBuf {
    dir.join(format!("{}.json", id))
}

fn read_config_file(path: &PathBuf) -> Result<Config, AppError> {
    let content = fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::ConfigNotFound {
                name: path.to_string_lossy().to_string(),
            }
        } else {
            AppError::IoError {
                message: e.to_string(),
            }
        }
    })?;
    serde_json::from_str(&content).map_err(AppError::from)
}

pub fn list_configs() -> Result<Vec<ConfigMeta>, AppError> {
    let dir = effective_configs_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut metas = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        match read_config_file(&path) {
            Ok(cfg) => metas.push(ConfigMeta {
                id: cfg.id,
                label: cfg.label,
                updated_at: cfg.updated_at,
            }),
            Err(_) => continue,
        }
    }
    metas.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
    Ok(metas)
}

pub fn get_config(id: &str) -> Result<Config, AppError> {
    let dir = effective_configs_dir()?;
    let path = config_path(id, &dir);
    read_config_file(&path)
}

pub fn create_config(label: &str) -> Result<Config, AppError> {
    let dir = effective_configs_dir()?;
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let payload = ConfigPayload {
        label: label.to_string(),
        provider: ConfigProvider::default(),
        agents: HashMap::new(),
        categories: HashMap::new(),
        source: None,
    };
    let config = Config {
        id: id.clone(),
        label: label.to_string(),
        created_at: now,
        updated_at: now,
        payload,
    };
    let path = config_path(&id, &dir);
    let content = serde_json::to_string_pretty(&config)?;
    fs::write(&path, content)?;
    Ok(config)
}

pub fn update_config(id: &str, payload: ConfigPayload) -> Result<Config, AppError> {
    let dir = effective_configs_dir()?;
    let path = config_path(id, &dir);
    let mut config = read_config_file(&path)?;
    let now = Utc::now();
    config.label = payload.label.clone();
    config.updated_at = now;
    config.payload = payload;
    let content = serde_json::to_string_pretty(&config)?;
    fs::write(&path, content)?;
    Ok(config)
}

pub fn delete_config(id: &str) -> Result<(), AppError> {
    let dir = effective_configs_dir()?;
    let path = config_path(id, &dir);
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

/// 复制配置：新 id、新 label（"原名 - Copy"），payload 深拷贝，source 清空
pub fn duplicate_config(id: &str) -> Result<Config, AppError> {
    let source = get_config(id)?;
    let new_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let new_label = format!("{} - Copy", source.label);
    let mut new_payload = source.payload;
    new_payload.label = new_label.clone();
    new_payload.source = None;

    let config = Config {
        id: new_id.clone(),
        label: new_label,
        created_at: now,
        updated_at: now,
        payload: new_payload,
    };
    let dir = effective_configs_dir()?;
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    let path = config_path(&new_id, &dir);
    let content = serde_json::to_string_pretty(&config)?;
    fs::write(&path, content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_temp_dir<F>(f: F)
    where
        F: FnOnce(PathBuf),
    {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().to_path_buf();
        TEST_CONFIGS_DIR.with(|cell| {
            *cell.borrow_mut() = Some(path.clone());
        });
        f(path.clone());
        TEST_CONFIGS_DIR.with(|cell| {
            *cell.borrow_mut() = None;
        });
    }

    #[test]
    fn test_create_and_list() {
        with_temp_dir(|_dir| {
            let created = create_config("test-config").unwrap();
            assert!(!created.id.is_empty());
            assert_eq!(created.label, "test-config");

            let list = list_configs().unwrap();
            assert_eq!(list.len(), 1);
            assert_eq!(list[0].id, created.id);
            assert_eq!(list[0].label, "test-config");
        });
    }

    #[test]
    fn test_create_multiple() {
        with_temp_dir(|_dir| {
            create_config("config-a").unwrap();
            create_config("config-b").unwrap();
            create_config("config-c").unwrap();

            let list = list_configs().unwrap();
            assert_eq!(list.len(), 3);
        });
    }

    #[test]
    fn test_get_existing() {
        with_temp_dir(|_dir| {
            let created = create_config("get-test").unwrap();
            let retrieved = get_config(&created.id).unwrap();
            assert_eq!(retrieved.label, "get-test");
            assert_eq!(retrieved.id, created.id);
        });
    }

    #[test]
    fn test_get_missing_returns_error() {
        with_temp_dir(|_dir| {
            let result = get_config("non-existent-uuid");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, AppError::ConfigNotFound { .. }));
        });
    }

    #[test]
    fn test_update_changes_label() {
        with_temp_dir(|_dir| {
            let created = create_config("old-label").unwrap();
            let new_payload = ConfigPayload {
                label: "new-label".to_string(),
                provider: ConfigProvider::default(),
                agents: HashMap::new(),
                categories: HashMap::new(),
                source: None,
            };
            update_config(&created.id, new_payload).unwrap();
            let updated = get_config(&created.id).unwrap();
            assert_eq!(updated.label, "new-label");
        });
    }

    #[test]
    fn test_update_changes_payload() {
        with_temp_dir(|_dir| {
            let created = create_config("payload-test").unwrap();
            let mut agents = HashMap::new();
            agents.insert("coder".to_string(), "gpt-4o".to_string());
            let mut categories = HashMap::new();
            categories.insert("web".to_string(), "claude-3".to_string());
            let new_payload = ConfigPayload {
                label: "payload-test".to_string(),
                provider: ConfigProvider::default(),
                agents,
                categories,
                source: None,
            };
            update_config(&created.id, new_payload).unwrap();
            let updated = get_config(&created.id).unwrap();
            assert_eq!(updated.payload.agents.get("coder"), Some(&"gpt-4o".to_string()));
            assert_eq!(updated.payload.categories.get("web"), Some(&"claude-3".to_string()));
        });
    }

    #[test]
    fn test_delete_removes_file() {
        with_temp_dir(|_dir| {
            let created = create_config("delete-me").unwrap();
            delete_config(&created.id).unwrap();
            let list = list_configs().unwrap();
            assert!(list.is_empty());
        });
    }

    #[test]
    fn test_delete_missing_is_ok() {
        with_temp_dir(|_dir| {
            let result = delete_config("totally-missing-uuid");
            assert!(result.is_ok());
        });
    }
}
