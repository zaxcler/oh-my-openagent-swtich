//! 激活状态检测模块
//!
//! 根据 `active.json` 记录和当前配置指纹，判断激活状态。

use crate::error::AppError;
use crate::storage::active::read_active;
use crate::storage::configs::ConfigMeta;

/// 激活状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActiveStatus {
    /// fingerprint 匹配，配置已激活
    Active { config_id: String },
    /// fingerprint 不匹配，外部已修改
    Drifted { config_id: String },
    /// active.json 缺失或损坏
    Unknown,
    /// 引用的 configId 在 configs/ 中不存在
    Orphan { reference_id: String },
}

use serde::{Deserialize, Serialize};

/// 检测当前激活状态
///
/// 参数：
/// - `configs`: 所有已知的配置元信息列表
/// - `opencode_fp`: 当前 opencode.jsonc 的 SHA256 指纹（`None` 表示文件不存在）
/// - `omos_fp`: 当前 oh-my-openagent.json 的 SHA256 指纹（`None` 表示文件不存在）
pub fn detect_active(
    configs: &[ConfigMeta],
    opencode_fp: Option<&str>,
    omos_fp: Option<&str>,
) -> Result<ActiveStatus, AppError> {
    let record = match read_active()? {
        None => return Ok(ActiveStatus::Unknown),
        Some(r) => r,
    };

    // 状态2：引用的 configId 不在 configs 列表中
    let config_ids: Vec<&str> = configs.iter().map(|c| c.id.as_str()).collect();
    if !config_ids.contains(&record.config_id.as_str()) {
        return Ok(ActiveStatus::Orphan {
            reference_id: record.config_id,
        });
    }

    // 状态3：fingerprint 不匹配
    if record.fingerprints.opencode != opencode_fp.unwrap_or("")
        || record.fingerprints.omos != omos_fp.unwrap_or("")
    {
        return Ok(ActiveStatus::Drifted {
            config_id: record.config_id,
        });
    }

    // 状态1：完全匹配
    Ok(ActiveStatus::Active {
        config_id: record.config_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::storage::active::{ActiveRecord, Fingerprints};

    fn make_meta(id: &str) -> ConfigMeta {
        ConfigMeta {
            id: id.to_string(),
            label: "test".to_string(),
            updated_at: Utc::now(),
        }
    }

    fn make_record(config_id: &str, opencode: &str, omos: &str) -> ActiveRecord {
        ActiveRecord {
            config_id: config_id.to_string(),
            applied_at: Utc::now(),
            fingerprints: Fingerprints {
                opencode: opencode.to_string(),
                omos: omos.to_string(),
            },
        }
    }

    // helper: 临时写入一条 active record
    fn with_active_record<F>(record: ActiveRecord, f: F)
    where
        F: FnOnce(),
    {
        use crate::storage::active::{write_active, TEST_ACTIVE_PATH};
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let path = tmp_dir.path().join("active.json");
        TEST_ACTIVE_PATH.with(|cell| {
            *cell.borrow_mut() = Some(path.clone());
        });
        write_active(&record).unwrap();
        f();
        TEST_ACTIVE_PATH.with(|cell| {
            *cell.borrow_mut() = None;
        });
    }

    #[test]
    fn test_detect_active_when_match() {
        let record = make_record("config-001", "fp_a", "fp_b");
        with_active_record(record, || {
            let configs = vec![make_meta("config-001"), make_meta("config-002")];
            let result = detect_active(&configs, Some("fp_a"), Some("fp_b")).unwrap();
            assert_eq!(result, ActiveStatus::Active {
                config_id: "config-001".to_string()
            });
        });
    }

    #[test]
    fn test_detect_drifted_when_mismatch() {
        let record = make_record("config-001", "fp_old", "fp_b");
        with_active_record(record, || {
            let configs = vec![make_meta("config-001")];
            let result = detect_active(&configs, Some("fp_new"), Some("fp_b")).unwrap();
            assert_eq!(result, ActiveStatus::Drifted {
                config_id: "config-001".to_string()
            });
        });
    }
}
