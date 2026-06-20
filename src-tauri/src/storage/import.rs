//! 导入模块
//!
//! - `read_current_opencode`: 从当前 opencode.jsonc 读取 provider.omos 块
//! - `read_oh_my_openagent`: 从 oh-my-openagent.json 读取 agents/categories
//! - `import_config_file`: 从外部 JSON 文件导入配置
//! - `auto_import_from_opencode`: 自动合并 opencode + oh-my-openagent 为 Config

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::config::jsonc::parse_jsonc;
use crate::error::AppError;
use crate::storage::configs::{create_config, update_config, Config, ConfigPayload, ConfigProvider, ModelEntry, ProviderOptions};
use crate::storage::paths::opencode_dir;
#[cfg(test)]
use crate::storage::paths::{set_test_opencode_dir, clear_test_opencode_dir};

/// 读取当前 opencode.jsonc 的 provider 下第一个 provider 块（作为 omos）
///
/// 规则：
/// - 优先读 `opencode_dir/opencode.jsonc`，不存在则读 `.json`
/// - 文件不存在 → `Ok(None)`
/// - 文件存在但 `provider` 下无任何 key → `Ok(None)`
/// - 取 provider 下第一个 key 的值（不管叫什么名字）
/// - 提取 `name` / `npm` / `options.apiKey` / `options.baseURL` / `models`
/// - 每个 model 只提取 `name`（必有）和 `group`（可选）
pub fn read_current_opencode() -> Result<Option<ConfigProvider>, AppError> {
    let dir = opencode_dir()?;

    let file_path = if dir.join("opencode.jsonc").exists() {
        dir.join("opencode.jsonc")
    } else if dir.join("opencode.json").exists() {
        dir.join("opencode.json")
    } else {
        return Ok(None);
    };

    let content = fs::read_to_string(&file_path)?;
    let value: Value = parse_jsonc(&content)?;

    // 取 provider 下第一个 key（不管叫什么名字）
    let provider_value = match value
        .get("provider")
        .and_then(|p| p.as_object())
        .and_then(|obj| obj.values().next())
    {
        Some(v) => v,
        None => return Ok(None),
    };

    let name = provider_value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let npm = provider_value
        .get("npm")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let options_value = provider_value.get("options");
    let api_key = options_value
        .and_then(|o| o.get("apiKey"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let base_url = options_value
        .and_then(|o| o.get("baseURL"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let mut models = HashMap::new();
    if let Some(models_value) = provider_value.get("models").and_then(|m| m.as_object()) {
        for (key, model_value) in models_value {
            let model_name = model_value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(key)
                .to_string();
            let group = model_value
                .get("group")
                .and_then(|v| v.as_str())
                .map(String::from);
            models.insert(key.clone(), ModelEntry { name: model_name, group });
        }
    }

    Ok(Some(ConfigProvider {
        name,
        npm,
        options: ProviderOptions { api_key, base_url },
        models,
    }))
}

/// 从 oh-my-openagent.json 读取 agents + categories
///
/// 规则：
/// - 优先读 `opencode_dir/oh-my-openagent.json`，不存在则读 `oh-my-opencode.json`（legacy）
/// - 文件不存在 → `Ok(None)`
/// - 每个 agent/category 的值取 `model` 字段，前缀替换为 `omos/`
pub fn read_oh_my_openagent() -> Result<Option<(HashMap<String, String>, HashMap<String, String>)>, AppError> {
    use crate::storage::paths::existing_omos_path;

    let file_path = match existing_omos_path()? {
        Some(p) => p,
        None => return Ok(None),
    };

    let content = fs::read_to_string(&file_path)?;
    let value: Value = serde_json::from_str(&content)?;

    fn to_omos_ref(raw: &str) -> String {
        let model_id = raw.rsplit('/').next().unwrap_or(raw);
        format!("omos/{}", model_id)
    }

    let agents = value
        .get("agents")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.get("model").and_then(|m| m.as_str()).map(|s| (k.clone(), to_omos_ref(s))))
                .collect()
        })
        .unwrap_or_default();

    let categories = value
        .get("categories")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.get("model").and_then(|m| m.as_str()).map(|s| (k.clone(), to_omos_ref(s))))
                .collect()
        })
        .unwrap_or_default();

    Ok(Some((agents, categories)))
}

/// 自动从 opencode + oh-my-openagent 导入配置
///
/// 合并策略：
/// - provider 信息来自 opencode.jsonc 的 provider.omos
/// - agents/categories 来自 oh-my-openagent.json
/// - 通过 source 字段去重
pub fn auto_import_from_opencode() -> Result<Option<Config>, AppError> {
    let provider_opt = read_current_opencode()?;
    let omos_opt = read_oh_my_openagent()?;

    if provider_opt.is_none() && omos_opt.is_none() {
        return Ok(None);
    }

    let provider = provider_opt.unwrap_or_default();
    let (agents, categories) = omos_opt.unwrap_or_default();

    // source 标记：记录两个源文件路径，用于后续去重
    let dir = opencode_dir()?;
    let opencode_path = dir.join("opencode.jsonc");
    let omos_path = dir.join("oh-my-openagent.json");
    let source = format!(
        "auto:{}|{}",
        opencode_path.display(),
        omos_path.display()
    );

    let existing_id = find_config_by_source(&source)?;

    let payload = ConfigPayload {
        label: "Auto-Imported".to_string(),
        provider,
        agents,
        categories,
        source: Some(source),
    };

    if let Some(id) = existing_id {
        let updated = update_config(&id, payload)?;
        Ok(Some(updated))
    } else {
        let config = create_config("Auto-Imported")?;
        let updated = update_config(&config.id, payload)?;
        Ok(Some(updated))
    }
}

fn find_config_by_source(source: &str) -> Result<Option<String>, AppError> {
    // 使用 effective_configs_dir 以支持测试覆盖（TEST_CONFIGS_DIR）
    let dir = {
        #[cfg(test)]
        {
            let override_path = crate::storage::configs::TEST_CONFIGS_DIR.with(|cell| (*cell.borrow()).clone());
            if let Some(path) = override_path {
                path
            } else {
                crate::storage::paths::configs_dir()?
            }
        }
        #[cfg(not(test))]
        {
            crate::storage::paths::configs_dir()?
        }
    };
    if !dir.exists() {
        return Ok(None);
    }
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let content = fs::read_to_string(&path)?;
        if let Ok(config) = serde_json::from_str::<Config>(&content) {
            if config.payload.source.as_deref() == Some(source) {
                return Ok(Some(config.id));
            }
        }
    }
    Ok(None)
}

/// 从外部 JSON 文件导入配置
///
/// 规则：
/// - 读 source JSON 文件
/// - 校验顶层结构：必须有 `label`、`provider`、`agents`、`categories`
/// - 校验失败 → `InvalidJson`
/// - 生成新 UUID，调用 `create_config` 写入 configs/
/// - 返回新 Config
pub fn import_config_file(source: &Path) -> Result<Config, AppError> {
    let content = fs::read_to_string(source)?;

    let value: Value = serde_json::from_str(&content)
        .map_err(|_| AppError::InvalidJson { path: source.display().to_string() })?;

    let label = match value.get("label").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            return Err(AppError::InvalidJson {
                path: format!("{} (missing or empty 'label')", source.display()),
            });
        }
    };

    let _ = value.get("provider").ok_or_else(|| AppError::InvalidJson {
        path: format!("{} (missing 'provider')", source.display()),
    })?;

    let agents_map: HashMap<String, String> = value
        .get("agents")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    let categories_map: HashMap<String, String> = value
        .get("categories")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    let provider_json = serde_json::to_string(value.get("provider").unwrap())
        .map_err(|_| AppError::InvalidJson { path: source.display().to_string() })?;
    let config_provider: ConfigProvider = serde_json::from_str(&provider_json)
        .map_err(|_| AppError::InvalidJson { path: source.display().to_string() })?;

    let mut config = create_config(&label)?;
    let payload = ConfigPayload {
        label: label.clone(),
        provider: config_provider,
        agents: agents_map,
        categories: categories_map,
        source: None,
    };
    config = update_config(&config.id, payload)?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::configs::TEST_CONFIGS_DIR;

    fn with_configs_dir<F>(f: F)
    where
        F: FnOnce(),
    {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().to_path_buf();
        TEST_CONFIGS_DIR.with(|cell| {
            *cell.borrow_mut() = Some(path);
        });
        f();
        TEST_CONFIGS_DIR.with(|cell| {
            *cell.borrow_mut() = None;
        });
        std::mem::forget(tmp);
    }

    #[test]
    fn test_read_opencode_missing() {
        with_configs_dir(|| {
            let tmp = tempfile::TempDir::new().unwrap();
            let fake_dir = tmp.path().to_path_buf();
            set_test_opencode_dir(fake_dir);

            let result = read_current_opencode();
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());

            clear_test_opencode_dir();
            std::mem::forget(tmp);
        });
    }

    #[test]
    fn test_read_opencode_no_omos() {
        with_configs_dir(|| {
            let tmp = tempfile::TempDir::new().unwrap();
            let opencode_dir = tmp.path().to_path_buf();
            set_test_opencode_dir(opencode_dir.clone());

            // provider 块完全为空
            std::fs::write(
                opencode_dir.join("opencode.jsonc"),
                r#"{
                    "provider": {}
                }"#,
            )
            .unwrap();

            let result = read_current_opencode();
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());

            clear_test_opencode_dir();
            std::mem::forget(tmp);
        });
    }

    #[test]
    fn test_read_opencode_picks_first_provider() {
        with_configs_dir(|| {
            let tmp = tempfile::TempDir::new().unwrap();
            let opencode_dir = tmp.path().to_path_buf();
            set_test_opencode_dir(opencode_dir.clone());

            // provider 下 key 不叫 omos，叫 openai，但 read_current_opencode 仍应取它
            std::fs::write(
                opencode_dir.join("opencode.jsonc"),
                r#"{
                    "provider": {
                        "openai": {
                            "name": "openai",
                            "npm": "@ai-sdk/openai-compatible",
                            "options": {
                                "apiKey": "sk-test",
                                "baseURL": "https://api.openai.com/v1"
                            },
                            "models": {
                                "gpt-4o": {"name": "GPT-4o"}
                            }
                        }
                    }
                }"#,
            )
            .unwrap();

            let result = read_current_opencode().unwrap();
            let provider = result.expect("expected Some(provider)");
            assert_eq!(provider.name, "openai");
            assert_eq!(provider.npm, "@ai-sdk/openai-compatible");
            assert_eq!(provider.models.len(), 1);

            clear_test_opencode_dir();
            std::mem::forget(tmp);
        });
    }

    #[test]
    fn test_read_opencode_with_omos() {
        with_configs_dir(|| {
            let tmp = tempfile::TempDir::new().unwrap();
            let opencode_dir = tmp.path().to_path_buf();
            set_test_opencode_dir(opencode_dir.clone());

            let jsonc_content = r#"{
                "provider": {
                    "omos": {
                        "name": "openai",
                        "npm": "@openai/plugin",
                        "options": {
                            "apiKey": "sk-test123",
                            "baseURL": "https://api.openai.com/v1"
                        },
                        "models": {
                            "gpt-4o": {
                                "name": "gpt-4o",
                                "group": "chat"
                            },
                            "gpt-4o-mini": {
                                "name": "gpt-4o-mini"
                            }
                        }
                    }
                }
            }"#;
            std::fs::write(opencode_dir.join("opencode.jsonc"), jsonc_content).unwrap();

            let result = read_current_opencode().unwrap();
            let provider = result.expect("expected Some(provider)");
            assert_eq!(provider.name, "openai");
            assert_eq!(provider.npm, "@openai/plugin");
            assert_eq!(provider.options.api_key, "sk-test123");
            assert_eq!(provider.options.base_url, "https://api.openai.com/v1");
            assert_eq!(provider.models.len(), 2);
            assert_eq!(provider.models.get("gpt-4o").unwrap().name, "gpt-4o");
            assert_eq!(
                provider.models.get("gpt-4o").unwrap().group,
                Some("chat".to_string())
            );
            assert_eq!(provider.models.get("gpt-4o-mini").unwrap().name, "gpt-4o-mini");
            assert_eq!(provider.models.get("gpt-4o-mini").unwrap().group, None);

            clear_test_opencode_dir();
            std::mem::forget(tmp);
        });
    }

    #[test]
    fn test_import_config_file() {
        with_configs_dir(|| {
            let tmp = tempfile::TempDir::new().unwrap();
            let source = tmp.path().join("source.json");
            let json = r#"{
                "label": "imported-config",
                "provider": {
                    "name": "anthropic",
                    "npm": "@anthropic/plugin",
                    "options": {
                        "api_key": "sk-ant-test",
                        "base_url": "https://api.anthropic.com"
                    },
                    "models": {
                        "claude-3-5-sonnet": {
                            "name": "claude-3-5-sonnet-20241022",
                            "group": "sonnet"
                        }
                    }
                },
                "agents": {
                    "coder": "claude-3-5-sonnet"
                },
                "categories": {
                    "default": "claude-3-5-sonnet"
                }
            }"#;
            std::fs::write(&source, json).unwrap();

            let config = import_config_file(&source).unwrap();
            assert_eq!(config.label, "imported-config");
            assert_eq!(config.payload.label, "imported-config");
            assert_eq!(config.payload.provider.name, "anthropic");
            assert_eq!(
                config.payload.agents.get("coder"),
                Some(&"claude-3-5-sonnet".to_string())
            );
            assert_eq!(
                config.payload.categories.get("default"),
                Some(&"claude-3-5-sonnet".to_string())
            );
            assert!(!config.id.is_empty());

            std::mem::forget(tmp);
        });
    }

    #[test]
    fn test_import_invalid_json() {
        with_configs_dir(|| {
            let tmp = tempfile::TempDir::new().unwrap();
            let source = tmp.path().join("invalid.json");
            std::fs::write(&source, "{ broken json }").unwrap();

            let result = import_config_file(&source);
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), AppError::InvalidJson { .. }));

            clear_test_opencode_dir();
            std::mem::forget(tmp);
        });
    }

    #[test]
    fn test_auto_import_creates_config() {
        with_configs_dir(|| {
            let tmp = tempfile::TempDir::new().unwrap();
            let opencode_dir = tmp.path().to_path_buf();
            set_test_opencode_dir(opencode_dir.clone());

            // provider key 不叫 omos，叫 openai（取第一个）
            std::fs::write(
                opencode_dir.join("opencode.jsonc"),
                r#"{
                    "provider": {
                        "openai": {
                            "name": "openai",
                            "npm": "@ai-sdk/openai-compatible",
                            "options": {
                                "apiKey": "sk-test",
                                "baseURL": "https://api.openai.com/v1"
                            },
                            "models": {
                                "gpt-4o": {"name": "GPT-4o"}
                            }
                        }
                    }
                }"#,
            )
            .unwrap();

            // oh-my-openagent 中 model 前缀是 openai/gpt-4o，应被替换为 omos/gpt-4o
            std::fs::write(
                opencode_dir.join("oh-my-openagent.json"),
                r#"{
                    "agents": {
                        "coder": {"model": "openai/gpt-4o"}
                    },
                    "categories": {
                        "web": {"model": "openai/gpt-4o"}
                    }
                }"#,
            )
            .unwrap();

            let result = auto_import_from_opencode().unwrap();
            assert!(result.is_some());
            let config = result.unwrap();
            assert_eq!(config.payload.provider.name, "openai");
            assert_eq!(config.payload.provider.options.api_key, "sk-test");
            assert_eq!(config.payload.agents.get("coder"), Some(&"omos/gpt-4o".to_string()));
            assert_eq!(config.payload.categories.get("web"), Some(&"omos/gpt-4o".to_string()));
            assert!(config.payload.source.is_some());

            clear_test_opencode_dir();
            std::mem::forget(tmp);
        });
    }

    #[test]
    fn test_auto_import_skips_if_exists() {
        with_configs_dir(|| {
            let tmp = tempfile::TempDir::new().unwrap();
            let opencode_dir = tmp.path().to_path_buf();
            set_test_opencode_dir(opencode_dir.clone());

            std::fs::write(
                opencode_dir.join("opencode.jsonc"),
                r#"{"provider": {"omos": {"name": "openai", "npm": "p", "options": {"apiKey": "k", "baseURL": "u"}, "models": {}}}}"#,
            )
            .unwrap();

            std::fs::write(
                opencode_dir.join("oh-my-openagent.json"),
                r#"{"agents": {}, "categories": {}}"#,
            )
            .unwrap();

            // 第一次导入
            let first = auto_import_from_opencode().unwrap();
            assert!(first.is_some());
            let first_config = first.unwrap();
            let first_id = first_config.id;

            // 第二次导入应更新而非创建新 config
            let second = auto_import_from_opencode().unwrap();
            assert!(second.is_some());
            assert_eq!(second.unwrap().id, first_id);

            clear_test_opencode_dir();
            std::mem::forget(tmp);
        });
    }

    #[test]
    fn test_auto_import_returns_none_when_empty() {
        with_configs_dir(|| {
            let tmp = tempfile::TempDir::new().unwrap();
            let opencode_dir = tmp.path().to_path_buf();
            set_test_opencode_dir(opencode_dir.clone());

            let result = auto_import_from_opencode().unwrap();
            assert!(result.is_none());

            clear_test_opencode_dir();
            std::mem::forget(tmp);
        });
    }
}
