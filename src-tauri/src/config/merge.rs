//! opencode.jsonc 合并策略 + oh-my-openagent.json 构建
//!
//! ## 合并语义
//!
//! `merge_opencode` 将一个 `ConfigPayload` 深度合并进目标 `opencode.jsonc` Value 中，
//! 保留用户已有的非 omos 字段（如 `plugin` / `permission` / `$schema` / 其他 provider）。
//!
//! `provider.omos` 块内：
//! - `name` / `npm` / `options`：整体替换
//! - `models`：深度合并；相同 model id 仅覆盖 `name` / `group`，保留
//!   `headers` / `limit` / `whitelist` / `attachment` / `cost` / `modalities` /
//!   `experimental` / `provider` 等用户已配置字段
//! - 不删除 target 中多余的 model（保守策略）
//!
//! `build_oh_my_openagent` 是整体替换式构建：直接产出完整的
//! `oh-my-openagent.json` Value，不再与磁盘现有内容做合并。

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use crate::error::AppError;
use crate::storage::configs::{ConfigPayload, ModelEntry, ProviderOptions};

/// 固定 $schema 引用地址
const OH_MY_OPENCODE_SCHEMA: &str =
    "https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/dev/assets/oh-my-opencode.schema.json";

/// omos provider 的固定 npm 包名
const OMOS_NPM: &str = "omos";

/// 深度合并 source ConfigPayload 到 target opencode.jsonc Value 的 provider.omos 块
///
/// 保留语义：
/// - target 中非 omos 字段（`plugin` / `permission` / `$schema`）原样保留
/// - target 中已有的 omos.models 其 `headers` / `limit` / `whitelist` / `attachment` /
///   `cost` / `modalities` / `experimental` / `provider` 等字段原样保留
///
/// 替换语义：
/// - target 中 `provider` 下除 `omos` 外的所有其他 key（如 `openai` / `anthropic` 等）被删除
pub fn merge_opencode(target: &mut Value, source: &ConfigPayload) -> Result<(), AppError> {
    if !target.is_object() {
        *target = Value::Object(Map::new());
    }
    let root = target.as_object_mut().expect("target 是 object 已被确保");

    if !root.get("provider").map(|v| v.is_object()).unwrap_or(false) {
        root.insert("provider".to_string(), Value::Object(Map::new()));
    }
    let providers = root
        .get_mut("provider")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| AppError::Unknown {
            message: "opencode.jsonc 缺少 provider 块且无法创建".into(),
        })?;

    let mut omos_block = Map::new();
    omos_block.insert("name".to_string(), Value::String(source.provider.name.clone()));
    omos_block.insert("npm".to_string(), Value::String(source.provider.npm.clone()));
    omos_block.insert("options".to_string(), build_options_value(&source.provider.options));

    // models 深度合并：先取 target 中已有 model（保留未知字段），再叠加 source
    let mut models_map = Map::new();
    if let Some(existing_models) = providers
        .get("omos")
        .and_then(|v| v.get("models"))
        .and_then(|v| v.as_object())
    {
        for (model_id, existing_value) in existing_models {
            models_map.insert(model_id.clone(), existing_value.clone());
        }
    }
    for (model_id, entry) in &source.provider.models {
        merge_single_model(&mut models_map, model_id, entry);
    }
    omos_block.insert("models".to_string(), Value::Object(models_map));

    // 清空 provider 下所有 key，只保留 omos
    let other_keys: Vec<String> = providers
        .keys()
        .filter(|k| k.as_str() != "omos")
        .cloned()
        .collect();
    for k in other_keys {
        providers.remove(&k);
    }
    providers.insert("omos".to_string(), Value::Object(omos_block));
    Ok(())
}

/// 构建整体替换的 oh-my-openagent.json Value
///
/// 格式：
/// ```json
/// {
///   "agents": {
///     "coder": { "model": "omos/gpt-4o" }
///   },
///   "categories": {
///     "web": { "model": "omos/gpt-4o" }
///   }
/// }
/// ```
///
/// model 引用固定用 `omos/` 前缀（与 opencode.jsonc 中的 `provider.omos` 对应）。
pub fn build_oh_my_openagent(payload: &ConfigPayload) -> Value {
    let mut agents = Map::new();
    for (k, v) in &payload.agents {
        let model_id = extract_model_id(v);
        let model_ref = format!("{}/{}", OMOS_NPM, model_id);
        agents.insert(k.clone(), json!({ "model": model_ref }));
    }
    let mut categories = Map::new();
    for (k, v) in &payload.categories {
        let model_id = extract_model_id(v);
        let model_ref = format!("{}/{}", OMOS_NPM, model_id);
        categories.insert(k.clone(), json!({ "model": model_ref }));
    }
    json!({
        "$schema": OH_MY_OPENCODE_SCHEMA,
        "agents": Value::Object(agents),
        "categories": Value::Object(categories),
    })
}

/// 从前端传入的 value 中提取 model id。
/// 前端 RoleSelect 的 value 可能是 "omos/xxx" 或纯 "xxx"。
fn extract_model_id(value: &str) -> &str {
    value.rsplit('/').next().unwrap_or(value)
}

fn build_options_value(opts: &ProviderOptions) -> Value {
    json!({
        "apiKey": opts.api_key,
        "baseURL": opts.base_url,
    })
}

/// 合并单个 model entry：仅覆盖 `name`，group 在 source 提供时替换，否则保留 target 原值（保守策略）
fn merge_single_model(
    models: &mut Map<String, Value>,
    model_id: &str,
    entry: &ModelEntry,
) {
    match models.get_mut(model_id) {
        None => {
            let mut obj = Map::new();
            obj.insert("name".to_string(), Value::String(entry.name.clone()));
            if let Some(group) = &entry.group {
                obj.insert("group".to_string(), Value::String(group.clone()));
            }
            models.insert(model_id.to_string(), Value::Object(obj));
        }
        Some(existing) => {
            let Some(obj) = existing.as_object_mut() else {
                let mut new_obj = Map::new();
                new_obj.insert("name".to_string(), Value::String(entry.name.clone()));
                if let Some(group) = &entry.group {
                    new_obj.insert("group".to_string(), Value::String(group.clone()));
                }
                *existing = Value::Object(new_obj);
                return;
            };
            obj.insert("name".to_string(), Value::String(entry.name.clone()));
            if let Some(group) = &entry.group {
                obj.insert("group".to_string(), Value::String(group.clone()));
            }
        }
    }
}

/// 抑制未使用导入告警（保留供后续 build 端到端测试时直接使用）
#[allow(dead_code)]
fn _hashmap_marker(_: &HashMap<String, String>) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::configs::{ConfigPayload, ConfigProvider, ModelEntry, ProviderOptions};
    use serde_json::json;
    use std::collections::HashMap;

    fn payload_with_one_model(
        name: &str,
        npm: &str,
        api_key: &str,
        base_url: &str,
        model_id: &str,
        model_name: &str,
        group: Option<&str>,
    ) -> ConfigPayload {
        let mut models = HashMap::new();
        models.insert(
            model_id.to_string(),
            ModelEntry {
                name: model_name.to_string(),
                group: group.map(|s| s.to_string()),
            },
        );
        ConfigPayload {
            label: "test-payload".to_string(),
            provider: ConfigProvider {
                name: name.to_string(),
                npm: npm.to_string(),
                options: ProviderOptions {
                    api_key: api_key.to_string(),
                    base_url: base_url.to_string(),
                },
                models,
            },
            agents: HashMap::new(),
            categories: HashMap::new(),
            source: None,
        }
    }

    // ---------- 1. target.m1 有 headers → 保留 ----------
    #[test]
    fn test_merge_preserves_target_model_headers() {
        let mut target = json!({
            "provider": {
                "omos": {
                    "name": "old",
                    "npm": "old-npm",
                    "options": {"apiKey": "old-key", "baseURL": "https://old.example.com"},
                    "models": {
                        "m1": {
                            "name": "old-m1-name",
                            "headers": {"X-Custom": "keep-me"}
                        }
                    }
                }
            }
        });
        let source = payload_with_one_model(
            "OpenAI", "@ai-sdk/openai", "new-key", "https://new.example.com",
            "m1", "new-m1-name", None,
        );

        merge_opencode(&mut target, &source).unwrap();

        let m1 = &target["provider"]["omos"]["models"]["m1"];
        assert_eq!(m1["name"], "new-m1-name", "name 应被 source 覆盖");
        assert_eq!(
            m1["headers"]["X-Custom"], "keep-me",
            "target 中 headers 必须保留"
        );
    }

    // ---------- 2. target.m1 有 limit → 保留 ----------
    #[test]
    fn test_merge_preserves_target_model_limit() {
        let mut target = json!({
            "provider": {
                "omos": {
                    "name": "old",
                    "npm": "old-npm",
                    "options": {"apiKey": "k", "baseURL": "u"},
                    "models": {
                        "m1": {
                            "name": "old-m1",
                            "limit": {"context": 128000, "output": 8192}
                        }
                    }
                }
            }
        });
        let source = payload_with_one_model(
            "OpenAI", "@ai-sdk/openai", "k", "u", "m1", "new-m1", None,
        );

        merge_opencode(&mut target, &source).unwrap();

        let m1 = &target["provider"]["omos"]["models"]["m1"];
        assert_eq!(m1["name"], "new-m1");
        assert_eq!(m1["limit"]["context"], 128000, "limit 必须保留");
        assert_eq!(m1["limit"]["output"], 8192);
    }

    // ---------- 3. target.m1 有 group → 保留 ----------
    #[test]
    fn test_merge_preserves_target_model_group() {
        let mut target = json!({
            "provider": {
                "omos": {
                    "name": "old",
                    "npm": "old-npm",
                    "options": {"apiKey": "k", "baseURL": "u"},
                    "models": {
                        "m1": {
                            "name": "old-m1",
                            "group": "keep-this-group"
                        }
                    }
                }
            }
        });
        let source = payload_with_one_model(
            "OpenAI", "@ai-sdk/openai", "k", "u", "m1", "new-m1", None,
        );

        merge_opencode(&mut target, &source).unwrap();

        let m1 = &target["provider"]["omos"]["models"]["m1"];
        assert_eq!(m1["name"], "new-m1");
        assert_eq!(
            m1["group"], "keep-this-group",
            "source 未给 group 时必须保留 target 原 group"
        );
    }

    // ---------- 4. target 有 m1+m2，source 只有 m1 → 保留 m2（保守） ----------
    #[test]
    fn test_merge_keeps_extra_models_in_target() {
        let mut target = json!({
            "provider": {
                "omos": {
                    "name": "old",
                    "npm": "old-npm",
                    "options": {"apiKey": "k", "baseURL": "u"},
                    "models": {
                        "m1": {"name": "m1-name"},
                        "m2": {"name": "m2-name", "headers": {"X-Keep": "yes"}}
                    }
                }
            }
        });
        let source = payload_with_one_model(
            "OpenAI", "@ai-sdk/openai", "k", "u", "m1", "m1-new", None,
        );

        merge_opencode(&mut target, &source).unwrap();

        let models = &target["provider"]["omos"]["models"];
        assert_eq!(models["m1"]["name"], "m1-new");
        assert_eq!(models["m2"]["name"], "m2-name", "target.m2 不能被删除");
        assert_eq!(models["m2"]["headers"]["X-Keep"], "yes");
    }

    // ---------- 5. options 整块替换 ----------
    #[test]
    fn test_merge_options_full_replace() {
        let mut target = json!({
            "provider": {
                "omos": {
                    "name": "old",
                    "npm": "old-npm",
                    "options": {
                        "apiKey": "old-key",
                        "baseURL": "https://old.example.com",
                        "timeout": 30000
                    },
                    "models": {}
                }
            }
        });
        let source = payload_with_one_model(
            "OpenAI", "@ai-sdk/openai", "new-key", "https://new.example.com",
            "m1", "m1-name", None,
        );

        merge_opencode(&mut target, &source).unwrap();

        let opts = &target["provider"]["omos"]["options"];
        assert_eq!(opts["apiKey"], "new-key", "apiKey 整体替换");
        assert_eq!(opts["baseURL"], "https://new.example.com");
        assert!(
            opts.get("timeout").is_none(),
            "options 整体替换，target 中额外 timeout 字段应丢失（预期）"
        );
    }

    // ---------- 6. target 有 anthropic provider → 替换时删除，只保留 omos ----------
    #[test]
    fn test_merge_removes_other_providers() {
        let mut target = json!({
            "provider": {
                "anthropic": {
                    "name": "Anthropic",
                    "npm": "@ai-sdk/anthropic",
                    "options": {"apiKey": "anthro-key"}
                },
                "omos": {
                    "name": "old",
                    "npm": "old-npm",
                    "options": {"apiKey": "k", "baseURL": "u"},
                    "models": {}
                }
            }
        });
        let source = payload_with_one_model(
            "OpenAI", "@ai-sdk/openai", "k", "u", "m1", "m1", None,
        );

        merge_opencode(&mut target, &source).unwrap();

        let providers = &target["provider"];
        assert!(
            providers.get("anthropic").is_none(),
            "anthropic provider 应被删除"
        );
        assert_eq!(providers["omos"]["name"], "OpenAI");
    }

    // ---------- 7. target 有 $schema → 保留 ----------
    #[test]
    fn test_merge_preserves_schema() {
        let mut target = json!({
            "$schema": "https://example.com/some-other.schema.json",
            "provider": {
                "omos": {
                    "name": "old",
                    "npm": "old-npm",
                    "options": {"apiKey": "k", "baseURL": "u"},
                    "models": {}
                }
            }
        });
        let source = payload_with_one_model(
            "OpenAI", "@ai-sdk/openai", "k", "u", "m1", "m1", None,
        );

        merge_opencode(&mut target, &source).unwrap();

        assert_eq!(
            target["$schema"], "https://example.com/some-other.schema.json",
            "target 顶层 $schema 必须保留"
        );
    }

    // ---------- 8. target 有 plugin → 保留 ----------
    #[test]
    fn test_merge_preserves_plugin() {
        let mut target = json!({
            "plugin": ["some-plugin-1", "some-plugin-2"],
            "provider": {
                "omos": {
                    "name": "old",
                    "npm": "old-npm",
                    "options": {"apiKey": "k", "baseURL": "u"},
                    "models": {}
                }
            }
        });
        let source = payload_with_one_model(
            "OpenAI", "@ai-sdk/openai", "k", "u", "m1", "m1", None,
        );

        merge_opencode(&mut target, &source).unwrap();

        let plugin = target["plugin"].as_array().expect("plugin 应为 array");
        assert_eq!(plugin.len(), 2);
        assert_eq!(plugin[0], "some-plugin-1");
        assert_eq!(plugin[1], "some-plugin-2");
    }

    // ---------- 9. target 有 permission → 保留 ----------
    #[test]
    fn test_merge_preserves_permission() {
        let mut target = json!({
            "permission": {"bash": "allow", "edit": "deny"},
            "provider": {
                "omos": {
                    "name": "old",
                    "npm": "old-npm",
                    "options": {"apiKey": "k", "baseURL": "u"},
                    "models": {}
                }
            }
        });
        let source = payload_with_one_model(
            "OpenAI", "@ai-sdk/openai", "k", "u", "m1", "m1", None,
        );

        merge_opencode(&mut target, &source).unwrap();

        let perm = &target["permission"];
        assert_eq!(perm["bash"], "allow");
        assert_eq!(perm["edit"], "deny");
    }

    // ---------- 10. source == target 时值不变 ----------
    #[test]
    fn test_merge_identical_no_change() {
        let mut target = json!({
            "$schema": "https://example.com/schema.json",
            "plugin": ["p1"],
            "provider": {
                "omos": {
                    "name": "OpenAI",
                    "npm": "@ai-sdk/openai",
                    "options": {"apiKey": "key-1", "baseURL": "https://api.example.com"},
                    "models": {
                        "m1": {"name": "Model One", "group": "g1"}
                    }
                }
            }
        });
        let snapshot_before = target.clone();
        let source = payload_with_one_model(
            "OpenAI",
            "@ai-sdk/openai",
            "key-1",
            "https://api.example.com",
            "m1",
            "Model One",
            Some("g1"),
        );

        merge_opencode(&mut target, &source).unwrap();

        assert_eq!(target, snapshot_before, "source 与 target 相同则结果应保持一致");
    }

    // ---------- 11. source 新增 model → target 出现 ----------
    #[test]
    fn test_merge_adds_new_model() {
        let mut target = json!({
            "provider": {
                "omos": {
                    "name": "old",
                    "npm": "old-npm",
                    "options": {"apiKey": "k", "baseURL": "u"},
                    "models": {
                        "m1": {"name": "m1-name"}
                    }
                }
            }
        });
        let source = payload_with_one_model(
            "OpenAI", "@ai-sdk/openai", "k", "u", "m_new", "new-model", Some("g-new"),
        );

        merge_opencode(&mut target, &source).unwrap();

        let models = &target["provider"]["omos"]["models"];
        assert_eq!(models["m1"]["name"], "m1-name", "原有 model 仍在");
        assert_eq!(models["m_new"]["name"], "new-model", "新 model 应被加入");
        assert_eq!(models["m_new"]["group"], "g-new");
    }

    // ---------- 12. target.m1 有 experimental / modalities → 保留 ----------
    #[test]
    fn test_merge_preserves_unknown_fields() {
        let mut target = json!({
            "provider": {
                "omos": {
                    "name": "old",
                    "npm": "old-npm",
                    "options": {"apiKey": "k", "baseURL": "u"},
                    "models": {
                        "m1": {
                            "name": "old-m1",
                            "experimental": {"thinking": true},
                            "modalities": {"input": ["text", "image"], "output": ["text"]},
                            "provider": {"npm": "@custom/pkg"}
                        }
                    }
                }
            }
        });
        let source = payload_with_one_model(
            "OpenAI", "@ai-sdk/openai", "k", "u", "m1", "new-m1", None,
        );

        merge_opencode(&mut target, &source).unwrap();

        let m1 = &target["provider"]["omos"]["models"]["m1"];
        assert_eq!(m1["name"], "new-m1");
        assert_eq!(m1["experimental"]["thinking"], true, "experimental 必须保留");
        assert_eq!(m1["modalities"]["input"][1], "image", "modalities 必须保留");
        assert_eq!(m1["provider"]["npm"], "@custom/pkg", "provider 子字段必须保留");
    }

    // ---------- build_oh_my_openagent 基础 ----------
    #[test]
    fn test_build_oh_my_openagent_basic_shape() {
        let mut agents = HashMap::new();
        agents.insert("coder".to_string(), "gpt-4o".to_string());
        let mut categories = HashMap::new();
        categories.insert("web".to_string(), "claude-3".to_string());
        let payload = ConfigPayload {
            label: "label".to_string(),
            provider: ConfigProvider {
                name: "openai".to_string(),
                ..ConfigProvider::default()
            },
            agents,
            categories,
            source: None,
        };

        let v = build_oh_my_openagent(&payload);

        assert_eq!(v["$schema"], OH_MY_OPENCODE_SCHEMA);
        // model 引用固定用 omos/ 前缀，与 opencode.jsonc 的 provider.omos 对应
        assert_eq!(v["agents"]["coder"]["model"], "omos/gpt-4o");
        assert_eq!(v["categories"]["web"]["model"], "omos/claude-3");
    }
}
