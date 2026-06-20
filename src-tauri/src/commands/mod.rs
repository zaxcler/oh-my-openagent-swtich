//! Tauri 命令层
//!
//! 将 Wave 1-2 的 Rust 功能封装为 12 个 `#[tauri::command]`，供前端调用。
//!
//! 每个命令只做三件事：参数校验 → 调模块 → 返回 `Result<T, AppError>`

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::fingerprint;
use crate::config::jsonc::parse_jsonc;
use crate::config::{build_oh_my_openagent, merge_opencode};
use crate::error::AppError;
use crate::storage::active::{ActiveRecord, Fingerprints, write_active};
use crate::storage::atomic::atomic_write_json;
use crate::storage::backup::{backup_file, delete_backup as delete_backup_item, list_backups as list_backup_items, restore_backup as restore_backup_item, BackupMeta};
use crate::storage::configs::{
    Config, ConfigMeta, ConfigPayload,
};
use crate::storage::detect::{detect_active, ActiveStatus};
use crate::storage::export::export_config as export_config_impl;
use crate::storage::import::{import_config_file as import_config_file_impl, read_current_opencode};
use crate::storage::paths::{existing_omos_path, omos_path, opencode_dir};

// ---------------------------------------------------------------------------
// 辅助类型
// ---------------------------------------------------------------------------

/// apply_config 的返回值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResult {
    /// 本次备份的文件路径列表
    pub backup_files: Vec<PathBuf>,
    /// 应用时间
    pub applied_at: DateTime<Utc>,
    /// opencode.jsonc 是否被修改
    pub opencode_updated: bool,
    /// oh-my-openagent.json 是否被修改
    pub omos_updated: bool,
}

// ---------------------------------------------------------------------------
// 命令 1-5: 配置 CRUD
// ---------------------------------------------------------------------------

/// 列出所有配置元信息（按 updated_at 降序）
#[tauri::command]
pub fn list_configs() -> Result<Vec<ConfigMeta>, AppError> {
    crate::storage::configs::list_configs()
}

/// 读取单个配置文件
#[tauri::command]
pub fn get_config(id: String) -> Result<Config, AppError> {
    crate::storage::configs::get_config(&id)
}

/// 创建新配置（初始 payload 为默认值）
#[tauri::command]
pub fn create_config(label: String) -> Result<Config, AppError> {
    crate::storage::configs::create_config(&label)
}

/// 更新配置（完整替换 payload）
#[tauri::command]
pub fn update_config(id: String, payload: ConfigPayload) -> Result<Config, AppError> {
    crate::storage::configs::update_config(&id, payload)
}

/// 删除配置
#[tauri::command]
pub fn delete_config(id: String) -> Result<(), AppError> {
    crate::storage::configs::delete_config(&id)
}

/// 复制配置：创建新 config，label = "原名 - Copy"，payload 深拷贝，source 清空
#[tauri::command]
pub fn duplicate_config(id: String) -> Result<Config, AppError> {
    crate::storage::configs::duplicate_config(&id)
}

// ---------------------------------------------------------------------------
// 命令 6: apply_config — 核心流程
// ---------------------------------------------------------------------------

/// 将配置应用到 opencode 环境
///
/// 流程（严格按顺序）：
/// 1. get_config(id) 拿 Config，不存在 → ConfigNotFound
/// 2. backup_file(opencode.jsonc) — 备份 opencode 配置
/// 3. backup_file(oh-my-openagent.json) — 备份 omos 配置
/// 4. 读 opencode.jsonc → 解析为 serde_json::Value
/// 5. merge_opencode(opencode_value, &config.payload)
/// 6. atomic_write_json(opencode_path, &opencode_value)
/// 7. build_oh_my_openagent(&config.payload) → omos_value
/// 8. atomic_write_json(omos_path, &omos_value)
/// 9. write_active(ActiveRecord { config_id, applied_at, fingerprints })
/// 10. 返回 ApplyResult
///
/// 任何一步失败：备份已完成的文件保留，写入失败不更新 active.json
#[tauri::command]
pub fn apply_config(id: String) -> Result<ApplyResult, AppError> {
    // Step 1: 获取配置
    let config = crate::storage::configs::get_config(&id)?;

    let opencode_dir = opencode_dir()?;
    let opencode_path = opencode_dir.join("opencode.jsonc");
    let omos_path = omos_path()?;

    let mut backup_files: Vec<PathBuf> = Vec::new();

    // Step 2: 备份 opencode.jsonc（文件不存在则跳过）
    if opencode_path.exists() {
        match backup_file(&opencode_path) {
            Ok(path) => backup_files.push(path),
            Err(e) => {
                return Err(AppError::BackupFailed {
                    reason: format!("备份 opencode.jsonc 失败: {}", e),
                });
            }
        }
    }

    // Step 3: 备份 oh-my-openagent.json（文件不存在则跳过）
    if omos_path.exists() {
        match backup_file(&omos_path) {
            Ok(path) => backup_files.push(path),
            Err(e) => {
                return Err(AppError::BackupFailed {
                    reason: format!("备份 oh-my-openagent.json 失败: {}", e),
                });
            }
        }
    }

    // Step 4: 读取 opencode.jsonc
    let opencode_value: Value = if opencode_path.exists() {
        let content = fs::read_to_string(&opencode_path)?;
        parse_jsonc(&content)?
    } else {
        Value::Object(serde_json::Map::new())
    };

    let mut opencode_value = opencode_value;

    // Step 5: 深度合并
    merge_opencode(&mut opencode_value, &config.payload)?;

    // Step 6: 原子写入 opencode.jsonc
    atomic_write_json(&opencode_path, &opencode_value)?;

    // Step 7: 构建 omos Value
    let omos_value = build_oh_my_openagent(&config.payload);

    // Step 8: 原子写入 oh-my-openagent.json
    atomic_write_json(&omos_path, &omos_value)?;

    // Step 9: 写入 active.json
    let applied_at = Utc::now();
    let record = ActiveRecord {
        config_id: id.clone(),
        applied_at,
        fingerprints: Fingerprints {
            opencode: fingerprint(&opencode_value),
            omos: fingerprint(&omos_value),
        },
    };
    write_active(&record)?;

    // Step 10: 返回结果
    Ok(ApplyResult {
        backup_files,
        applied_at,
        opencode_updated: true,
        omos_updated: true,
    })
}

// ---------------------------------------------------------------------------
// 命令 7-9: 导入导出
// ---------------------------------------------------------------------------

/// 从当前 opencode.jsonc 读取 provider.omos 块，保存为新 Config
///
/// - opencode.jsonc 不存在 → Ok(None)
/// - 无 provider.omos → Ok(None)
/// - 正常导入 → Ok(Some(Config))
#[tauri::command]
pub fn import_from_opencode() -> Result<Option<Config>, AppError> {
    let provider = read_current_opencode()?;
    let Some(provider) = provider else {
        return Ok(None);
    };

        let agents = HashMap::new();
        let categories = HashMap::new();
    let label = format!("Imported-{}", Utc::now().format("%Y%m%d%H%M%S"));

    let config = crate::storage::configs::create_config(&label)?;
    let payload = ConfigPayload {
        label: label.clone(),
        provider,
        agents,
        categories,
        source: None,
    };
    let updated = crate::storage::configs::update_config(&config.id, payload)?;
    Ok(Some(updated))
}

/// 自动从 opencode.jsonc + oh-my-openagent.json 导入配置（app 启动时调用）
///
/// 合并 provider.omos + agents/categories，通过 source 字段去重。
/// 如果两个文件都不存在或为空 → Ok(None)
#[tauri::command]
pub fn auto_import_from_opencode() -> Result<Option<Config>, AppError> {
    crate::storage::import::auto_import_from_opencode()
}

/// 读取外部 oh-my-openagent 格式 JSON 文件，提取 agents + categories
///
/// 返回 (agents, categories)，每个值是纯 model id（去掉原 prefix），与 `agents.${key}` 表单值一致。
/// 如果文件不存在或解析失败 → AppError
#[tauri::command]
pub fn read_role_json_file(path: String) -> Result<RoleJsonContent, AppError> {
    use std::fs;
    use serde_json::Value;

    let p = PathBuf::from(&path);
    let content = fs::read_to_string(&p).map_err(|e| AppError::IoError {
        message: e.to_string(),
    })?;
    let value: Value = serde_json::from_str(&content)?;

    fn strip_prefix(raw: &str) -> String {
        raw.rsplit('/').next().unwrap_or(raw).to_string()
    }

    let to_map = |obj: Option<&serde_json::Map<String, Value>>| -> HashMap<String, String> {
        obj.map(|m| {
            m.iter()
                .filter_map(|(k, v)| {
                    v.get("model")
                        .and_then(|m| m.as_str())
                        .map(|s| (k.clone(), strip_prefix(s)))
                })
                .collect()
        })
        .unwrap_or_default()
    };

    let agents = to_map(value.get("agents").and_then(|v| v.as_object()));
    let categories = to_map(value.get("categories").and_then(|v| v.as_object()));

    Ok(RoleJsonContent { agents, categories })
}

#[derive(serde::Serialize)]
pub struct RoleJsonContent {
    pub agents: HashMap<String, String>,
    pub categories: HashMap<String, String>,
}

/// 从外部 JSON 文件导入配置
#[tauri::command]
pub fn import_config_file(path: String) -> Result<Config, AppError> {
    import_config_file_impl(PathBuf::from(path).as_path())
}

/// 导出配置到目标路径
#[tauri::command]
pub fn export_config(id: String, target: String) -> Result<(), AppError> {
    let config = crate::storage::configs::get_config(&id)?;
    export_config_impl(&config, PathBuf::from(target).as_path())
}

// ---------------------------------------------------------------------------
// 命令 10-11: 备份管理
// ---------------------------------------------------------------------------

/// 列出所有备份（按 created_at 降序）
#[tauri::command]
pub fn list_backups() -> Result<Vec<BackupMeta>, AppError> {
    list_backup_items()
}

/// 从备份还原到原路径
#[tauri::command]
pub fn restore_backup(filename: String) -> Result<(), AppError> {
    restore_backup_item(&filename)
}

/// 删除指定备份文件（含侧车）
#[tauri::command]
pub fn delete_backup(filename: String) -> Result<(), AppError> {
    delete_backup_item(&filename)
}

// ---------------------------------------------------------------------------
// 命令 12: 激活状态检测
// ---------------------------------------------------------------------------

/// 检测激活状态
///
/// - 读取 active.json
/// - 计算当前 opencode.jsonc 和 oh-my-openagent.json 的指纹
/// - 与 active.json 中的指纹对比
/// - 返回 ActiveStatus 枚举
#[tauri::command]
pub fn get_active_status(configs: Vec<ConfigMeta>) -> Result<ActiveStatus, AppError> {
    let opencode_dir = match opencode_dir() {
        Ok(d) => d,
        Err(_) => return Ok(ActiveStatus::Unknown),
    };

    let opencode_path = opencode_dir.join("opencode.jsonc");
    // 兼容 rename transition：先 oh-my-opencode.json 后 oh-my-openagent.json
    let omos_path = existing_omos_path()?.unwrap_or_else(|| omos_path().unwrap());

    let opencode_fp: Option<String> = fs::read_to_string(&opencode_path)
        .ok()
        .and_then(|c| parse_jsonc(&c).ok())
        .map(|v| fingerprint(&v));

    let omos_fp: Option<String> = fs::read_to_string(&omos_path)
        .ok()
        .and_then(|c| serde_json::from_str::<Value>(&c).ok())
        .map(|v| fingerprint(&v));

    detect_active(&configs, opencode_fp.as_deref(), omos_fp.as_deref())
}

// ---------------------------------------------------------------------------
// 集成测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::storage::active::TEST_ACTIVE_PATH;
    use crate::storage::backup::TEST_BACKUPS_DIR;
    use crate::storage::configs::TEST_CONFIGS_DIR;
    use crate::storage::paths::{set_test_opencode_dir, clear_test_opencode_dir};
    use std::collections::HashMap;

    fn setup_all_test_dirs() -> tempfile::TempDir {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        let configs_dir = root.join("configs");
        let backups_dir = root.join("backups");
        let opencode_dir = root.join("opencode");
        let active_path = root.join("active.json");

        fs::create_dir_all(&configs_dir).unwrap();
        fs::create_dir_all(&backups_dir).unwrap();
        fs::create_dir_all(&opencode_dir).unwrap();

        TEST_CONFIGS_DIR.with(|cell| *cell.borrow_mut() = Some(configs_dir));
        TEST_BACKUPS_DIR.with(|cell| *cell.borrow_mut() = Some(backups_dir));
        set_test_opencode_dir(opencode_dir);
        TEST_ACTIVE_PATH.with(|cell| *cell.borrow_mut() = Some(active_path));

        tmp
    }

    fn teardown_all_test_dirs() {
        TEST_CONFIGS_DIR.with(|cell| *cell.borrow_mut() = None);
        TEST_BACKUPS_DIR.with(|cell| *cell.borrow_mut() = None);
        clear_test_opencode_dir();
        TEST_ACTIVE_PATH.with(|cell| *cell.borrow_mut() = None);
    }

    fn get_opencode_dir() -> PathBuf {
        crate::storage::paths::opencode_dir().unwrap()
    }

    /// 测试 1: CRUD 完整流程
    #[test]
    fn test_commands_list_get_create_update_delete() {
        let _tmp = setup_all_test_dirs();

        // list — 初始为空
        let metas = list_configs().unwrap();
        assert!(metas.is_empty());

        // create
        let config = create_config("my-config".to_string()).unwrap();
        assert_eq!(config.label, "my-config");
        assert!(!config.id.is_empty());

        // list — 应该有 1 个
        let metas = list_configs().unwrap();
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].id, config.id);

        // get
        let retrieved = get_config(config.id.clone()).unwrap();
        assert_eq!(retrieved.label, "my-config");

        // update
        let mut agents = HashMap::new();
        agents.insert("coder".to_string(), "gpt-4o".to_string());
        let payload = ConfigPayload {
            label: "my-config".to_string(),
            provider: crate::storage::configs::ConfigProvider::default(),
            agents,
            categories: HashMap::new(),
            source: None,
        };
        let updated = update_config(config.id.clone(), payload).unwrap();
        assert_eq!(updated.label, "my-config");
        assert_eq!(updated.payload.agents.get("coder"), Some(&"gpt-4o".to_string()));

        // delete
        delete_config(config.id.clone()).unwrap();
        let metas = list_configs().unwrap();
        assert!(metas.is_empty());

        teardown_all_test_dirs();
    }

    /// 测试 2: apply_config 写入文件
    #[test]
    fn test_apply_config_writes_files() {
        let _tmp = setup_all_test_dirs();

        let opencode_dir = get_opencode_dir();
        let opencode_path = opencode_dir.join("opencode.jsonc");
        let omos_path = opencode_dir.join("oh-my-openagent.json");

        // 写入原始 opencode.jsonc
        let original_content = r#"{
            "$schema": "https://example.com/schema.json",
            "plugin": ["some-plugin"]
        }"#;
        fs::write(&opencode_path, original_content).unwrap();

        // 创建配置
        let mut agents = HashMap::new();
        agents.insert("coder".to_string(), "gpt-4o".to_string());
        let mut categories = HashMap::new();
        categories.insert("default".to_string(), "gpt-4o".to_string());
        let payload = ConfigPayload {
            label: "apply-test".to_string(),
            provider: crate::storage::configs::ConfigProvider {
                name: "openai".to_string(),
                npm: "@ai-sdk/openai".to_string(),
                options: crate::storage::configs::ProviderOptions {
                    api_key: "sk-test".to_string(),
                    base_url: "https://api.openai.com".to_string(),
                },
                models: HashMap::new(),
            },
            agents,
            categories,
            source: None,
        };
        let config = create_config("apply-test".to_string()).unwrap();
        let updated = update_config(config.id.clone(), payload).unwrap();

        // apply
        let result = apply_config(updated.id.clone()).unwrap();
        assert!(result.opencode_updated);
        assert!(result.omos_updated);
        assert!(result.applied_at <= chrono::Utc::now());
        assert!(!result.backup_files.is_empty());

        // 验证 opencode.jsonc 被修改
        let new_content = fs::read_to_string(&opencode_path).unwrap();
        // serde_json to_string_pretty adds space after colon: "name" : "openai"
        assert!(new_content.contains("openai"), "opencode.jsonc 应包含 omos name");
        assert!(new_content.contains("\"plugin\""), "opencode.jsonc 应保留原有 plugin 字段");

        // 验证 oh-my-openagent.json 被写入
        assert!(omos_path.exists(), "oh-my-openagent.json 应被创建");
        let omos_content = fs::read_to_string(&omos_path).unwrap();
        assert!(omos_content.contains("\"model\""), "omos agents 应为对象格式");
        assert!(omos_content.contains("omos/gpt-4o"), "omos model 引用应固定用 omos/ 前缀");

        teardown_all_test_dirs();
    }

    /// 测试 3: import_from_opencode 从 fixture 导入
    #[test]
    fn test_import_from_opencode() {
        let _tmp = setup_all_test_dirs();

        let opencode_dir = get_opencode_dir();
        let opencode_path = opencode_dir.join("opencode.jsonc");

        // 写入 fixture opencode.jsonc
        let fixture = r#"{
            "provider": {
                "omos": {
                    "name": "anthropic",
                    "npm": "@anthropic/anthropic-sdk",
                    "options": {
                        "apiKey": "sk-ant-test",
                        "baseURL": "https://api.anthropic.com"
                    },
                    "models": {
                        "claude-3-5-sonnet": {
                            "name": "claude-3-5-sonnet-20241022",
                            "group": "sonnet"
                        }
                    }
                }
            }
        }"#;
        fs::write(&opencode_path, fixture).unwrap();

        // import
        let result = import_from_opencode().unwrap();
        assert!(result.is_some());
        let config = result.unwrap();

        assert_eq!(config.payload.provider.name, "anthropic");
        assert_eq!(config.payload.provider.npm, "@anthropic/anthropic-sdk");
        assert_eq!(
            config.payload.provider.options.api_key,
            "sk-ant-test"
        );
        assert_eq!(
            config.payload.provider.models.get("claude-3-5-sonnet").unwrap().name,
            "claude-3-5-sonnet-20241022"
        );

        teardown_all_test_dirs();
    }

    /// 测试 4: get_active_status 返回正确状态
    #[test]
    fn test_get_active_status() {
        let _tmp = setup_all_test_dirs();

        let opencode_dir = get_opencode_dir();
        let opencode_path = opencode_dir.join("opencode.jsonc");
        let omos_path = opencode_dir.join("oh-my-openagent.json");

        // 写文件并计算指纹
        fs::write(&opencode_path, r#"{"agents":{}}"#).unwrap();
        fs::write(&omos_path, r#"{"agents":{}}"#).unwrap();

        let opencode_fp = {
            let content = fs::read_to_string(&opencode_path).unwrap();
            let value: Value = parse_jsonc(&content).unwrap();
            fingerprint(&value)
        };
        let omos_fp = {
            let content = fs::read_to_string(&omos_path).unwrap();
            let value: Value = serde_json::from_str(&content).unwrap();
            fingerprint(&value)
        };

        // 写入 active.json（匹配）
        let record = ActiveRecord {
            config_id: "config-001".to_string(),
            applied_at: Utc::now(),
            fingerprints: Fingerprints {
                opencode: opencode_fp.clone(),
                omos: omos_fp.clone(),
            },
        };
        crate::storage::active::write_active(&record).unwrap();

        let configs = vec![ConfigMeta {
            id: "config-001".to_string(),
            label: "test".to_string(),
            updated_at: Utc::now(),
        }];

        // Active
        let status = get_active_status(configs.clone()).unwrap();
        assert_eq!(status, ActiveStatus::Active { config_id: "config-001".to_string() });

        // Drifted — 修改文件
        fs::write(&opencode_path, r#"{"agents":{"coder":"gpt-4o"}}"#).unwrap();
        let status = get_active_status(configs.clone()).unwrap();
        assert_eq!(status, ActiveStatus::Drifted { config_id: "config-001".to_string() });

        // Orphan — config 不在列表中
        let status = get_active_status(vec![]).unwrap();
        assert_eq!(status, ActiveStatus::Orphan { reference_id: "config-001".to_string() });

        // Unknown — 删除 active.json
        crate::storage::active::clear_active().unwrap();
        let status = get_active_status(configs).unwrap();
        assert_eq!(status, ActiveStatus::Unknown);

        teardown_all_test_dirs();
    }
}
