//! JSONC 解析与序列化工具

use crate::error::AppError;
use jsonc_parser::parse_to_serde_value;
use serde_json::Value;

/// 解析 JSONC 字符串为 serde_json::Value
#[allow(dead_code)]
pub fn parse_jsonc(s: &str) -> Result<Value, AppError> {
    parse_to_serde_value(s, &Default::default())
        .map_err(|e| AppError::JsoncParse { path: e.to_string() })?
        .ok_or_else(|| AppError::JsoncParse { path: "空文档".into() })
}

/// 去掉 JSONC 注释，返回纯 JSON 字符串
#[allow(dead_code)]
pub fn strip_jsonc_comments(s: &str) -> Result<String, AppError> {
    let value = parse_jsonc(s)?;
    serde_json::to_string(&value).map_err(|e| e.into())
}

/// 美化序列化 serde_json::Value
#[allow(dead_code)]
pub fn pretty_json(value: &Value) -> Result<String, AppError> {
    serde_json::to_string_pretty(value).map_err(|e| e.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_comments() {
        let jsonc = r#"{
            // 这是一个注释
            "name": "test",
            // 另一行注释
            "value": 42
        }"#;
        let value = parse_jsonc(jsonc).unwrap();
        assert_eq!(value["name"], "test");
        assert_eq!(value["value"], 42);
    }

    #[test]
    fn test_parse_with_trailing_comma() {
        let jsonc = r#"{
            "a": 1,
            "b": 2,
        }"#;
        let value = parse_jsonc(jsonc).unwrap();
        assert_eq!(value["a"], 1);
        assert_eq!(value["b"], 2);
    }

    #[test]
    fn test_parse_invalid_jsonc() {
        let jsonc = r#"{"broken"#;
        let result = parse_jsonc(jsonc);
        assert!(result.is_err());
    }

    #[test]
    fn test_round_trip() {
        let jsonc = r#"{"name":"test","value":42}"#;
        let v1 = parse_jsonc(jsonc).unwrap();
        let pretty = pretty_json(&v1).unwrap();
        let v2 = parse_jsonc(&pretty).unwrap();
        assert_eq!(v1, v2);
    }
}
