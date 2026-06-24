/**
 * 前端类型定义 —— 必须与 Rust 端完全一致。
 *
 * 数据源：
 * - `src-tauri/src/storage/configs.rs` (Config / ConfigPayload / ConfigProvider / ...)
 * - `src-tauri/src/storage/backup.rs`  (BackupMeta)
 * - `src-tauri/src/storage/detect.rs`  (ActiveStatus)
 * - `src-tauri/src/commands/mod.rs`    (ApplyResult)
 *
 * 修改本文件前请先同步 Rust 端，**禁止在前端重新定义**。
 */

// ----- ConfigPayload 嵌套结构 -----

/** opencode 支持的多模态类型。 */
export type Modality = 'text' | 'image' | 'audio' | 'video' | 'pdf';

/** 单个 model 的多模态声明。input/output 至少有一项非空时,会被写入 `opencode.jsonc` 的 `modalities` 块。 */
export interface Modalities {
  input: Modality[];
  output: Modality[];
}

/** 单个模型项；`group` 可选（部分 provider 不需要分组），`modalities` 可选（不填或全空时不会写入 JSON）。 */
export interface ModelEntry {
  name: string;
  group?: string;
  modalities?: Modalities;
}

/** provider 私有配置（API key / base URL 等）。 */
export interface ProviderOptions {
  api_key: string;
  base_url: string;
}

/** AI provider 定义（对应 opencode.json 的 provider 节点）。 */
export interface ConfigProvider {
  name: string;
  npm: string;
  options: ProviderOptions;
  models: Record<string, ModelEntry>;
}

/** 一个完整配置的有效载荷。 */
export interface ConfigPayload {
  label: string;
  provider: ConfigProvider;
  /** key = agent key，value = 模型 id。 */
  agents: Record<string, string>;
  /** key = category key，value = agent key。 */
  categories: Record<string, string>;
}

// ----- 顶层资源 -----

/** 单个配置（含完整 payload）。 */
export interface Config {
  id: string;
  label: string;
  created_at: string;
  updated_at: string;
  payload: ConfigPayload;
}

/** 列表接口返回的轻量元信息。 */
export interface ConfigMeta {
  id: string;
  label: string;
  updated_at: string;
}

/** 备份文件元信息。 */
export interface BackupMeta {
  filename: string;
  original_path: string;
  created_at: string;
  size_bytes: number;
}

/** `apply_config` 命令返回结果。 */
export interface ApplyResult {
  backup_files: string[];
  applied_at: string;
  opencode_updated: boolean;
  omos_updated: boolean;
}

/**
 * 当前激活状态枚举。
 *
 * 标签 `type` 标识变体；携带字段按变体不同而异。
 * - `Active`     : 完全匹配，已应用此 config
 * - `Drifted`    : 已被外部修改，需要重应用
 * - `Unknown`    : 无法判断（无 active 记录或 fingerprints 缺失）
 * - `Orphan`     : active 指向一个已被删除的 config
 */
export type ActiveStatus =
  | { type: 'Active'; config_id: string }
  | { type: 'Drifted'; config_id: string }
  | { type: 'Unknown' }
  | { type: 'Orphan'; reference_id: string };
