//! opencode.jsonc 合并策略 + oh-my-openagent.json 构建
//!
//! ## 合并语义
//!
//! `merge_opencode` 将一个 `ConfigPayload` 深度合并进目标 `opencode.jsonc` Value 中，
//! 保留用户已有的非 omos 字段（如 `plugin` / `permission` / `$schema` / 其他 provider）。
//!
//! `provider.omos` 块内：
//! - `name` / `npm` / `options`：整体替换
//! - `models`：全量替换，只保留 source 中的 models（不保留 target 已有 model）
//!
//! `build_oh_my_openagent` 是整体替换式构建：直接产出完整的
//! `oh-my-openagent.json` Value，不再与磁盘现有内容做合并。

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use crate::error::AppError;
use crate::storage::configs::{ConfigPayload, Modalities, ProviderOptions};

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

    // models 全量替换（不保留 target 已有 model）
    let mut models_map = Map::new();
    for (model_id, entry) in &source.provider.models {
        let mut obj = Map::new();
        obj.insert("name".to_string(), Value::String(entry.name.clone()));
        if let Some(group) = &entry.group {
            obj.insert("group".to_string(), Value::String(group.clone()));
        }
        if let Some(modalities) = build_modalities_value(&entry.modalities) {
            obj.insert("modalities".to_string(), modalities);
        }
        models_map.insert(model_id.clone(), Value::Object(obj));
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
    let case_map: HashMap<String, String> = payload
        .provider
        .models
        .keys()
        .map(|k| (k.to_lowercase(), k.clone()))
        .collect();

    fn omos_ref(v: &str, case_map: &HashMap<String, String>) -> String {
        let model_id = v.rsplit('/').next().unwrap_or(v);
        let resolved = case_map
            .get(&model_id.to_lowercase())
            .cloned()
            .unwrap_or_else(|| model_id.to_string());
        format!("{}/{}", OMOS_NPM, resolved)
    }

    let mut agents = Map::new();
    for (k, v) in &payload.agents {
        agents.insert(k.clone(), json!({ "model": omos_ref(v, &case_map) }));
    }
    let mut categories = Map::new();
    for (k, v) in &payload.categories {
        categories.insert(k.clone(), json!({ "model": omos_ref(v, &case_map) }));
    }
    json!({
        "$schema": OH_MY_OPENCODE_SCHEMA,
        "agents": Value::Object(agents),
        "categories": Value::Object(categories),
    })
}

fn build_options_value(opts: &ProviderOptions) -> Value {
    json!({
        "apiKey": opts.api_key,
        "baseURL": opts.base_url,
    })
}

/// 构造 modalities JSON 块。
///
/// - `None` 或 input/output 都为空的 modalities → 返回 `None`(由 caller 决定是否写入 `modalities` 键)
/// - 非空时,只输出非空数组(避免写入 `"input": []`)
fn build_modalities_value(m: &Option<Modalities>) -> Option<Value> {
    let m = m.as_ref()?;
    if m.input.is_empty() && m.output.is_empty() {
        return None;
    }
    let mut obj = Map::new();
    if !m.input.is_empty() {
        obj.insert(
            "input".to_string(),
            Value::Array(m.input.iter().map(|s| Value::String(s.clone())).collect()),
        );
    }
    if !m.output.is_empty() {
        obj.insert(
            "output".to_string(),
            Value::Array(m.output.iter().map(|s| Value::String(s.clone())).collect()),
        );
    }
    Some(Value::Object(obj))
}

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
                modalities: None,
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

    fn payload_with_modalities(
        model_id: &str,
        input: Vec<&str>,
        output: Vec<&str>,
    ) -> ConfigPayload {
        let mut models = HashMap::new();
        models.insert(
            model_id.to_string(),
            ModelEntry {
                name: format!("{}-name", model_id),
                group: None,
                modalities: Some(Modalities {
                    input: input.iter().map(|s| s.to_string()).collect(),
                    output: output.iter().map(|s| s.to_string()).collect(),
                }),
            },
        );
        ConfigPayload {
            label: "test-payload".to_string(),
            provider: ConfigProvider {
                name: "OpenAI".to_string(),
                npm: "@ai-sdk/openai".to_string(),
                options: ProviderOptions {
                    api_key: "k".to_string(),
                    base_url: "u".to_string(),
                },
                models,
            },
            agents: HashMap::new(),
            categories: HashMap::new(),
            source: None,
        }
    }

    // ---------- 1. options 整块替换 ----------
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

    // ---------- 2. target 有 anthropic provider → 替换时删除，只保留 omos ----------
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

    // ---------- 3. target 有 $schema → 保留 ----------
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

    // ---------- 4. target 有 plugin → 保留 ----------
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

    // ---------- 5. target 有 permission → 保留 ----------
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

    // ---------- 6. models 全量替换：target-only model 被移除 ----------
    #[test]
    fn test_merge_models_full_replacement() {
        let mut target = json!({
            "provider": {
                "omos": {
                    "name": "old",
                    "npm": "old-npm",
                    "options": {"apiKey": "k", "baseURL": "u"},
                    "models": {
                        "m1": {"name": "target-only-model"},
                        "m2": {"name": "m2-with-extra", "headers": {"X": "y"}}
                    }
                }
            }
        });
        let source = payload_with_one_model(
            "OpenAI", "@ai-sdk/openai", "k", "u", "new_m", "source-only-model", Some("g1"),
        );

        merge_opencode(&mut target, &source).unwrap();

        let models = target["provider"]["omos"]["models"].as_object().unwrap();
        assert_eq!(models.len(), 1, "应只有 1 个 model（source 的）");
        assert!(models.get("m1").is_none(), "target-only m1 应被移除");
        assert!(models.get("m2").is_none(), "target-only m2 应被移除");
        assert_eq!(models["new_m"]["name"], "source-only-model");
        assert_eq!(models["new_m"]["group"], "g1");
    }

    // ---------- 7. source == target 时值不变 ----------
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

    // ---------- 8. source 仅有 m1，target 有 m1+m2 → 最终只有 m1 ----------
    #[test]
    fn test_merge_removes_extra_target_models() {
        let mut target = json!({
            "provider": {
                "omos": {
                    "name": "old",
                    "npm": "old-npm",
                    "options": {"apiKey": "k", "baseURL": "u"},
                    "models": {
                        "m1": {"name": "m1-name"},
                        "m2": {"name": "m2-name"}
                    }
                }
            }
        });
        let source = payload_with_one_model(
            "OpenAI", "@ai-sdk/openai", "k", "u", "m1", "m1-updated", None,
        );

        merge_opencode(&mut target, &source).unwrap();

        let models = target["provider"]["omos"]["models"].as_object().unwrap();
        assert_eq!(models.len(), 1, "target 多余的 m2 应被移除");
        assert!(models.get("m2").is_none(), "m2 不应出现在 models 中");
        assert_eq!(models["m1"]["name"], "m1-updated", "m1 name 被 source 覆盖");
    }

    // ---------- build_oh_my_openagent：基础形状 + 大小写 + omos/ 前缀 ----------
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
        assert_eq!(v["agents"]["coder"]["model"], "omos/gpt-4o");
        assert_eq!(v["categories"]["web"]["model"], "omos/claude-3");
    }

    // ---------- build_oh_my_openagent：保持原大小写（输入=输出一致） ----------
    #[test]
    fn test_build_oh_my_openagent_preserves_model_id_case() {
        let mut agents = HashMap::new();
        agents.insert("coder".to_string(), "omos/MiniMax-M3".to_string());
        let payload = ConfigPayload {
            label: "label".to_string(),
            provider: ConfigProvider {
                name: "MiniMax".to_string(),
                ..ConfigProvider::default()
            },
            agents,
            categories: HashMap::new(),
            source: None,
        };

        let v = build_oh_my_openagent(&payload);

        assert_eq!(v["agents"]["coder"]["model"], "omos/MiniMax-M3", "应保持原大小写，不做转换");
    }

    // ---------- build_oh_my_openagent：大小写归一化（用 provider.models 的精确大小写） ----------
    #[test]
    fn test_build_oh_my_openagent_normalizes_case_via_provider_models() {
        let mut models = HashMap::new();
        models.insert("MiniMax-M3".to_string(), ModelEntry { name: "MiniMax M3".to_string(), group: None, modalities: None });
        models.insert("deepseek-v4-flash".to_string(), ModelEntry { name: "DeepSeek V4 Flash".to_string(), group: None, modalities: None });

        let mut agents = HashMap::new();
        agents.insert("multimodal-looker".to_string(), "omos/MiniMax-m3".to_string());
        agents.insert("explore".to_string(), "omos/DeepSeek-V4-Flash".to_string());
        let payload = ConfigPayload {
            label: "label".to_string(),
            provider: ConfigProvider {
                name: "demo".to_string(),
                options: ProviderOptions::default(),
                models,
                ..ConfigProvider::default()
            },
            agents,
            categories: HashMap::new(),
            source: None,
        };

        let v = build_oh_my_openagent(&payload);

        assert_eq!(v["agents"]["multimodal-looker"]["model"], "omos/MiniMax-M3", "应归一化为 provider.models 中的精确大小写");
        assert_eq!(v["agents"]["explore"]["model"], "omos/deepseek-v4-flash", "大写应归一化为小写");
    }

    // ---------- build_oh_my_openagent：未知 model 保持原样 ----------
    #[test]
    fn test_build_oh_my_openagent_unknown_model_keeps_original() {
        let mut models = HashMap::new();
        models.insert("deepseek-v4-flash".to_string(), ModelEntry { name: "DeepSeek V4 Flash".to_string(), group: None, modalities: None });

        let mut agents = HashMap::new();
        agents.insert("coder".to_string(), "omos/MiniMax-m3".to_string());
        let payload = ConfigPayload {
            label: "label".to_string(),
            provider: ConfigProvider {
                name: "demo".to_string(),
                options: ProviderOptions::default(),
                models,
                ..ConfigProvider::default()
            },
            agents,
            categories: HashMap::new(),
            source: None,
        };

        let v = build_oh_my_openagent(&payload);

        assert_eq!(v["agents"]["coder"]["model"], "omos/MiniMax-m3", "未知 model 应保持原样（让用户去修）");
    }

    // ---------- modalities: 同时有 input + output ----------
    #[test]
    fn test_merge_writes_modalities() {
        let mut target = json!({"provider": {"omos": {"name": "x", "npm": "x", "options": {}, "models": {}}}});
        let source = payload_with_modalities(
            "MiniMax-M3",
            vec!["text", "image"],
            vec!["text"],
        );

        merge_opencode(&mut target, &source).unwrap();

        let model = &target["provider"]["omos"]["models"]["MiniMax-M3"];
        assert_eq!(model["name"], "MiniMax-M3-name");
        assert_eq!(model["modalities"]["input"][0], "text");
        assert_eq!(model["modalities"]["input"][1], "image");
        assert_eq!(model["modalities"]["output"][0], "text");
    }

    // ---------- modalities: 仅 input,不输出 output 字段 ----------
    #[test]
    fn test_merge_modalities_input_only() {
        let mut target = json!({"provider": {"omos": {"name": "x", "npm": "x", "options": {}, "models": {}}}});
        let source = payload_with_modalities("m", vec!["text"], vec![]);

        merge_opencode(&mut target, &source).unwrap();

        let m = &target["provider"]["omos"]["models"]["m"]["modalities"];
        assert!(m.get("input").is_some());
        assert!(m.get("output").is_none(), "output 为空时不应输出空数组");
    }

    // ---------- modalities: 都为空时不写 modalities 字段 ----------
    #[test]
    fn test_merge_modalities_all_empty_no_field() {
        let mut target = json!({"provider": {"omos": {"name": "x", "npm": "x", "options": {}, "models": {}}}});
        let source = payload_with_modalities("m", vec![], vec![]);

        merge_opencode(&mut target, &source).unwrap();

        let model = &target["provider"]["omos"]["models"]["m"];
        assert!(
            model.get("modalities").is_none(),
            "input/output 都为空时不应写入 modalities 字段"
        );
    }

    // ---------- modalities: None 时不写 modalities 字段(向后兼容) ----------
    #[test]
    fn test_merge_modalities_none_no_field() {
        let mut target = json!({"provider": {"omos": {"name": "x", "npm": "x", "options": {}, "models": {}}}});
        let source = payload_with_one_model("OpenAI", "@ai-sdk/openai", "k", "u", "m", "m-name", None);

        merge_opencode(&mut target, &source).unwrap();

        let model = &target["provider"]["omos"]["models"]["m"];
        assert!(
            model.get("modalities").is_none(),
            "modalities 为 None 时(默认行为)不应写入字段,保证旧配置 JSON 完全一致"
        );
    }
}
