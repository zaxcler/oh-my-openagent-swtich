//! 配置指纹计算模块
//!
//! 使用 SHA256 对配置内容进行规范化后哈希，用于检测配置是否被外部修改。

use sha2::{Digest, Sha256};
use serde_json::Value;

/// 计算配置的 SHA256 指纹
///
/// 规范化流程：
/// 1. 序列化为字节（字段顺序由 serde 保证）
/// 2. 转为字符串并去除空白字符
/// 3. 计算 SHA256 并转为十六进制字符串
pub fn fingerprint(value: &Value) -> String {
    // serde_json::to_vec 保证字段顺序一致
    let bytes = serde_json::to_vec(value).expect("value should serialize");
    let json_str = String::from_utf8(bytes).expect("valid utf-8");
    // 去除所有空白字符（空格、换行、制表符）
    let normalized: String = json_str.chars().filter(|c| !c.is_whitespace()).collect();

    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_fingerprint_stable() {
        let v1 = json!({
            "agents": { "coder": "gpt-4o" },
            "categories": { "web": "claude-3" }
        });
        let v2 = json!({
            "agents": { "coder": "gpt-4o" },
            "categories": { "web": "claude-3" }
        });
        assert_eq!(fingerprint(&v1), fingerprint(&v2));
    }

    #[test]
    fn test_fingerprint_different_order() {
        // 值相同但字段顺序不同，应产生相同 hash
        let v1 = json!({
            "name": "alice",
            "age": 30,
            "city": "beijing"
        });
        let v2 = json!({
            "city": "beijing",
            "age": 30,
            "name": "alice"
        });
        assert_eq!(fingerprint(&v1), fingerprint(&v2));
    }
}
