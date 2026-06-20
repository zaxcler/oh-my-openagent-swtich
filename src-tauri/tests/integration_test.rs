//! 跨模块集成测试（T10）
//!
//! 验证 Wave 1-2 所有模块协同工作：commands、storage/configs、storage/import、
//! storage/backup、storage/active、config/merge。
//!
//! ## 测试钩子使用说明
//!
//! `storage::active::TEST_ACTIVE_PATH` 标了 `#[cfg(test)]`，integration test
//! 在 cargo 编译时**不会**给 lib crate 传 `--cfg test`，所以无法直接 `use`。
//! 因此采用平台特定的环境变量重定向策略，让 `dirs::config_dir()` 返回
//! tempdir 下的位置：
//!
//! - **macOS**: `dirs::config_dir()` 读 `$HOME`，所以设置 `HOME` 让它返回
//!   tempdir 下的 `Library/Application Support`。
//! - **Linux**: `dirs::config_dir()` 读 `$XDG_CONFIG_HOME` / `$HOME`，
//!   所以同时设置两者指向 tempdir。
//! - **Windows**: `dirs::config_dir()` 读 `%APPDATA%`，所以设置它指向 tempdir。
//!
//! `OMO_TEST_CONFIGS_DIR` / `OMO_TEST_BACKUPS_DIR` 覆盖 configs / backups 目录；
//! `set_test_opencode_dir` 覆盖 opencode 目录（thread-local）。环境变量是
//! 进程级共享的，用全局 `TEST_LOCK` 串行化访问。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use oh_my_openagent_switch_lib::commands::{
    apply_config, create_config, import_from_opencode, list_configs, update_config,
};
use oh_my_openagent_switch_lib::config::jsonc::parse_jsonc;
use oh_my_openagent_switch_lib::storage::configs::{
    ConfigPayload, ConfigProvider, ModelEntry, ProviderOptions,
};
use oh_my_openagent_switch_lib::storage::paths::{
    clear_test_opencode_dir, set_test_opencode_dir,
};
use serde_json::Value;

/// 全局串行化锁：保护 OMO_TEST_* / HOME / XDG_CONFIG_HOME / APPDATA
/// 等进程级环境变量的访问
static TEST_LOCK: Mutex<()> = Mutex::new(());

/// 测试夹具：创建隔离的临时目录并覆盖所有路径
struct TestEnv {
    /// 持有临时目录句柄，drop 时自动清理
    _tmp: tempfile::TempDir,
    /// 全局锁的 guard：测试期间持有，drop 时自动释放
    _guard: MutexGuard<'static, ()>,
    configs_dir: PathBuf,
    backups_dir: PathBuf,
    opencode_dir: PathBuf,
    active_path: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        // 串行化：避免多个测试同时设置环境变量
        let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::TempDir::new().expect("创建临时目录");
        let root = tmp.path();

        let configs_dir = root.join("configs");
        let backups_dir = root.join("backups");
        let opencode_dir = root.join("opencode");

        fs::create_dir_all(&configs_dir).expect("创建 configs 目录");
        fs::create_dir_all(&backups_dir).expect("创建 backups 目录");
        fs::create_dir_all(&opencode_dir).expect("创建 opencode 目录");

        // 通过环境变量覆盖 dirs::config_dir()，让 app_config_root()
        // 解析到 tempdir 下的某个子目录
        let app_root = setup_platform_config_root(root);
        let active_path = app_root.join("active.json");

        // 通过环境变量覆盖 configs 和 backups 目录
        // SAFETY: 已获取 TEST_LOCK 串行化
        unsafe {
            std::env::set_var("OMO_TEST_CONFIGS_DIR", &configs_dir);
            std::env::set_var("OMO_TEST_BACKUPS_DIR", &backups_dir);
        }

        // 通过 thread-local 覆盖 opencode 目录
        set_test_opencode_dir(opencode_dir.clone());

        Self {
            _tmp: tmp,
            _guard: guard,
            configs_dir,
            backups_dir,
            opencode_dir,
            active_path,
        }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        // 清理环境变量
        // SAFETY: 已持有 TEST_LOCK（同一线程 drop）
        unsafe {
            std::env::remove_var("OMO_TEST_CONFIGS_DIR");
            std::env::remove_var("OMO_TEST_BACKUPS_DIR");
        }
        clear_platform_config_root();
        clear_test_opencode_dir();
    }
}

// -----------------------------------------------------------------------------
// 平台特定：让 dirs::config_dir() 返回 tempdir 下的位置
// -----------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn setup_platform_config_root(root: &Path) -> PathBuf {
    // macOS: dirs::config_dir() = $HOME/Library/Application Support
    // app_config_root()  = $HOME/Library/Application Support/oh-my-openagent-switch
    // SAFETY: 持有 TEST_LOCK
    unsafe {
        std::env::set_var("HOME", root);
    }
    let app_root = root.join("Library/Application Support/oh-my-openagent-switch");
    fs::create_dir_all(&app_root).expect("创建 macOS app_config_root");
    app_root
}

#[cfg(target_os = "macos")]
fn clear_platform_config_root() {
    // SAFETY: 持有 TEST_LOCK
    unsafe {
        std::env::remove_var("HOME");
    }
}

#[cfg(target_os = "linux")]
fn setup_platform_config_root(root: &Path) -> PathBuf {
    // Linux: dirs::config_dir() = $XDG_CONFIG_HOME or $HOME/.config
    // app_config_root()  = <config_dir>/oh-my-openagent-switch
    // SAFETY: 持有 TEST_LOCK
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", root);
        std::env::set_var("HOME", root);
    }
    let app_root = root.join("oh-my-openagent-switch");
    fs::create_dir_all(&app_root).expect("创建 linux app_config_root");
    app_root
}

#[cfg(target_os = "linux")]
fn clear_platform_config_root() {
    // SAFETY: 持有 TEST_LOCK
    unsafe {
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::remove_var("HOME");
    }
}

#[cfg(target_os = "windows")]
fn setup_platform_config_root(root: &Path) -> PathBuf {
    // Windows: dirs::config_dir() = %APPDATA%
    // app_config_root()  = %APPDATA%/oh-my-openagent-switch
    // SAFETY: 持有 TEST_LOCK
    unsafe {
        std::env::set_var("APPDATA", root);
    }
    let app_root = root.join("oh-my-openagent-switch");
    fs::create_dir_all(&app_root).expect("创建 windows app_config_root");
    app_root
}

#[cfg(target_os = "windows")]
fn clear_platform_config_root() {
    // SAFETY: 持有 TEST_LOCK
    unsafe {
        std::env::remove_var("APPDATA");
    }
}

// -----------------------------------------------------------------------------
// 辅助：构造 ConfigPayload
// -----------------------------------------------------------------------------

/// 构造带 omos provider + agents + categories 的 ConfigPayload
fn make_provider_payload(label: &str, api_key: &str, base_url: &str) -> ConfigPayload {
    let mut models = HashMap::new();
    models.insert(
        "gpt-4o".to_string(),
        ModelEntry {
            name: "gpt-4o".to_string(),
            group: Some("chat".to_string()),
        },
    );
    let mut agents = HashMap::new();
    agents.insert("coder".to_string(), "gpt-4o".to_string());
    let mut categories = HashMap::new();
    categories.insert("default".to_string(), "gpt-4o".to_string());
    ConfigPayload {
        label: label.to_string(),
        provider: ConfigProvider {
            name: "openai".to_string(),
            npm: "@ai-sdk/openai".to_string(),
            options: ProviderOptions {
                api_key: api_key.to_string(),
                base_url: base_url.to_string(),
            },
            models,
        },
        agents,
        categories,
        source: None,
    }
}

// =============================================================================
// 测试 1: create → list → 看到新建的 config
// =============================================================================
#[test]
fn test_create_config_and_list() {
    let env = TestEnv::new();

    // 初始 list 应为空
    let initial = list_configs().expect("list_configs 初始调用");
    assert!(initial.is_empty(), "初始状态应无 config，实际: {initial:?}");

    // create 一个 config
    let config = create_config("my-config".to_string()).expect("create_config");
    assert_eq!(config.label, "my-config");
    assert!(!config.id.is_empty(), "config id 不应为空");

    // list 应包含 1 个
    let listed = list_configs().expect("list_configs 调用");
    assert_eq!(listed.len(), 1, "list 应有 1 个 config");
    assert_eq!(listed[0].id, config.id);
    assert_eq!(listed[0].label, "my-config");

    // config 文件实际写到 configs 目录
    let expected_file = env.configs_dir.join(format!("{}.json", config.id));
    assert!(
        expected_file.exists(),
        "config 文件应存在: {}",
        expected_file.display()
    );
    let content = fs::read_to_string(&expected_file).expect("读 config 文件");
    let value: Value = serde_json::from_str(&content).expect("parse config JSON");
    assert_eq!(value["label"], "my-config");
    assert_eq!(value["id"], config.id);
}

// =============================================================================
// 测试 2: apply_config 写入 opencode.jsonc + oh-my-openagent.json + active.json
// =============================================================================
#[test]
fn test_apply_config_writes_opencode_and_active() {
    let env = TestEnv::new();

    // 准备含 plugin 字段的 opencode.jsonc（验证保留语义）
    let opencode_path = env.opencode_dir.join("opencode.jsonc");
    let omos_path = env.opencode_dir.join("oh-my-openagent.json");
    fs::write(
        &opencode_path,
        r#"{
            "plugin": ["some-plugin"]
        }"#,
    )
    .expect("写 opencode.jsonc");

    // create + update
    let config = create_config("apply-test".to_string()).expect("create");
    let payload = make_provider_payload("apply-test", "sk-test-123", "https://api.openai.com/v1");
    let config = update_config(config.id.clone(), payload).expect("update");

    // apply
    let result = apply_config(config.id.clone()).expect("apply");
    assert!(result.opencode_updated, "opencode 应被更新");
    assert!(result.omos_updated, "omos 应被更新");
    assert!(!result.backup_files.is_empty(), "至少应有一个备份文件");

    // 验证 opencode.jsonc 包含 omos provider
    let opencode_content = fs::read_to_string(&opencode_path).expect("读 opencode.jsonc");
    let opencode_value: Value = parse_jsonc(&opencode_content).expect("parse opencode");
    assert_eq!(opencode_value["provider"]["omos"]["name"], "openai");
    assert_eq!(
        opencode_value["provider"]["omos"]["options"]["apiKey"],
        "sk-test-123"
    );
    assert_eq!(
        opencode_value["provider"]["omos"]["options"]["baseURL"],
        "https://api.openai.com/v1"
    );
    assert_eq!(
        opencode_value["provider"]["omos"]["models"]["gpt-4o"]["name"],
        "gpt-4o"
    );
    // 保留原 plugin 字段
    assert_eq!(opencode_value["plugin"][0], "some-plugin");

    // 验证 oh-my-openagent.json
    assert!(omos_path.exists(), "oh-my-openagent.json 应被创建");
    let omos_content = fs::read_to_string(&omos_path).expect("读 omos");
    let omos_value: Value = serde_json::from_str(&omos_content).expect("parse omos");
    assert_eq!(omos_value["agents"]["coder"]["model"], "omos/gpt-4o");
    assert_eq!(omos_value["categories"]["default"]["model"], "omos/gpt-4o");
    assert!(
        omos_value["$schema"].is_string(),
        "$schema 应为字符串"
    );

    // 验证 active.json
    assert!(env.active_path.exists(), "active.json 应被创建");
    let active_content = fs::read_to_string(&env.active_path).expect("读 active");
    let active_value: Value = serde_json::from_str(&active_content).expect("parse active");
    assert_eq!(active_value["config_id"], config.id);
    assert!(
        active_value["fingerprints"]["opencode"].is_string(),
        "opencode fingerprint 应存在"
    );
    assert!(
        active_value["fingerprints"]["omos"].is_string(),
        "omos fingerprint 应存在"
    );
}

// =============================================================================
// 测试 3: apply 后 backups/ 目录出现新备份文件
// =============================================================================
#[test]
fn test_apply_config_creates_backup() {
    let env = TestEnv::new();

    // 准备初始 opencode.jsonc
    let opencode_path = env.opencode_dir.join("opencode.jsonc");
    fs::write(&opencode_path, r#"{"initial": "v1"}"#).expect("写 opencode");

    // create config
    let config = create_config("backup-test".to_string()).expect("create");
    let payload = make_provider_payload("backup-test", "sk-key", "https://example.com");
    let config = update_config(config.id.clone(), payload).expect("update");

    // 第一次 apply：备份 opencode.jsonc
    let result = apply_config(config.id.clone()).expect("apply");
    assert!(!result.backup_files.is_empty(), "应有至少一个备份文件");

    // 验证 backups/ 目录有备份文件
    let backups: Vec<PathBuf> = fs::read_dir(&env.backups_dir)
        .expect("read backups dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();

    assert!(!backups.is_empty(), "backups/ 应有 .json 备份文件");
    assert_eq!(
        backups.len(),
        result.backup_files.len(),
        "实际备份数应与 ApplyResult 一致"
    );

    // 验证备份文件内容（保留原 opencode 内容）
    let first_backup = &backups[0];
    let backup_content = fs::read_to_string(first_backup).expect("读备份");
    let backup_value: Value = serde_json::from_str(&backup_content).expect("parse backup");
    assert_eq!(
        backup_value["initial"], "v1",
        "备份应保留原 opencode 内容"
    );

    // 验证侧车文件（记录原路径）
    let sidecar_name = format!(
        "{}.original_path",
        first_backup.file_name().unwrap().to_string_lossy()
    );
    let sidecar = first_backup.with_file_name(&sidecar_name);
    assert!(sidecar.exists(), "侧车文件应存在: {}", sidecar.display());
    let original = fs::read_to_string(&sidecar).expect("读 sidecar");
    assert!(
        original.contains("opencode.jsonc"),
        "sidecar 应记录原 opencode.jsonc 路径，实际: {original}"
    );

    // 第二次 apply：产生新备份
    fs::write(&opencode_path, r#"{"initial": "v2"}"#).expect("写 v2");
    let result2 = apply_config(config.id.clone()).expect("第二次 apply");
    assert!(
        !result2.backup_files.is_empty(),
        "第二次 apply 应至少备份 opencode.jsonc"
    );

    // 第二次 apply 的备份是当时（v2）的状态
    let new_backup = &result2.backup_files[0];
    let new_content = fs::read_to_string(new_backup).expect("读新备份");
    let new_value: Value = serde_json::from_str(&new_content).expect("parse 新备份");
    assert_eq!(
        new_value["initial"], "v2",
        "新备份应包含第二次 apply 当时的 v2 状态"
    );

    let backups_after: Vec<PathBuf> = fs::read_dir(&env.backups_dir)
        .expect("read backups dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    assert!(
        backups_after.len() > backups.len(),
        "第二次 apply 后 backups/ 应有更多文件，before={} after={}",
        backups.len(),
        backups_after.len()
    );
}

// =============================================================================
// 测试 4: 合并时删除非 omos provider
// =============================================================================
#[test]
fn test_merge_removes_other_providers() {
    let env = TestEnv::new();

    // 准备含 anthropic provider 的 opencode.jsonc
    let opencode_path = env.opencode_dir.join("opencode.jsonc");
    let initial = r#"{
        "provider": {
            "anthropic": {
                "name": "Anthropic",
                "npm": "@ai-sdk/anthropic",
                "options": {
                    "apiKey": "anthro-key-original"
                }
            }
        }
    }"#;
    fs::write(&opencode_path, initial).expect("写 opencode");

    // create + update（注入 openai omos）
    let config = create_config("merge-test".to_string()).expect("create");
    let payload = make_provider_payload("merge-test", "sk-openai-key", "https://api.openai.com/v1");
    let config = update_config(config.id.clone(), payload).expect("update");

    // apply
    apply_config(config.id.clone()).expect("apply");

    // 验证 anthropic 被删除，omos 被新 config 替换
    let content = fs::read_to_string(&opencode_path).expect("读 opencode");
    let value: Value = parse_jsonc(&content).expect("parse");

    assert!(
        value["provider"].get("anthropic").is_none(),
        "anthropic provider 应被删除"
    );

    // omos 被新 config 替换
    assert_eq!(value["provider"]["omos"]["name"], "openai");
    assert_eq!(
        value["provider"]["omos"]["options"]["apiKey"],
        "sk-openai-key"
    );
    assert_eq!(
        value["provider"]["omos"]["options"]["baseURL"],
        "https://api.openai.com/v1"
    );
    assert!(
        value["provider"]["omos"]["models"]["gpt-4o"].is_object(),
        "omos models.gpt-4o 应存在"
    );
}

// =============================================================================
// 测试 5: import_from_opencode 往返
// =============================================================================
#[test]
fn test_import_from_opencode_roundtrip() {
    let env = TestEnv::new();

    // 准备含 provider.omos 的 opencode.jsonc
    let opencode_path = env.opencode_dir.join("opencode.jsonc");
    let fixture = r#"{
        "provider": {
            "omos": {
                "name": "anthropic",
                "npm": "@anthropic/anthropic-sdk",
                "options": {
                    "apiKey": "sk-ant-test-123",
                    "baseURL": "https://api.anthropic.com"
                },
                "models": {
                    "claude-3-5-sonnet": {
                        "name": "claude-3-5-sonnet-20241022",
                        "group": "sonnet"
                    },
                    "claude-3-haiku": {
                        "name": "claude-3-haiku-20240307"
                    }
                }
            }
        }
    }"#;
    fs::write(&opencode_path, fixture).expect("写 opencode");

    // import_from_opencode
    let result = import_from_opencode().expect("import");
    let config = result.expect("应返回 Some(config)");

    // 验证 label 以 "Imported-" 开头
    assert!(
        config.label.starts_with("Imported-"),
        "label 应以 Imported- 开头，实际: {}",
        config.label
    );

    // 验证 provider 字段正确填充
    let provider = &config.payload.provider;
    assert_eq!(provider.name, "anthropic");
    assert_eq!(provider.npm, "@anthropic/anthropic-sdk");
    assert_eq!(provider.options.api_key, "sk-ant-test-123");
    assert_eq!(provider.options.base_url, "https://api.anthropic.com");

    // 验证 models 字段填充
    assert_eq!(provider.models.len(), 2, "应有 2 个 model");

    let sonnet = provider
        .models
        .get("claude-3-5-sonnet")
        .expect("claude-3-5-sonnet model");
    assert_eq!(sonnet.name, "claude-3-5-sonnet-20241022");
    assert_eq!(sonnet.group, Some("sonnet".to_string()));

    let haiku = provider
        .models
        .get("claude-3-haiku")
        .expect("claude-3-haiku model");
    assert_eq!(haiku.name, "claude-3-haiku-20240307");
    assert_eq!(haiku.group, None, "haiku 无 group 应为 None");

    // 验证 import 后 configs/ 中有对应 config 文件
    let expected_file = env.configs_dir.join(format!("{}.json", config.id));
    assert!(
        expected_file.exists(),
        "import 后 config 文件应存在: {}",
        expected_file.display()
    );
}
