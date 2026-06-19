/**
 * 11 个 agent key 及其展示标签。
 *
 * 来源：`oh-my-opencode` 项目的 agent 列表。
 * 顺序与官方文档保持一致，方便用户对照。
 */
export const AGENT_KEYS = [
  'sisyphus',
  'hephaestus',
  'oracle',
  'atlas',
  'metis',
  'momus',
  'multimodal-looker',
  'sisyphus-junior',
  'prometheus',
  'librarian',
  'explore',
] as const;

export type AgentKey = (typeof AGENT_KEYS)[number];

/** 8 个 category key 及其展示标签。 */
export const CATEGORY_KEYS = [
  'visual-engineering',
  'ultrabrain',
  'deep',
  'unspecified-high',
  'artistry',
  'writing',
  'quick',
  'unspecified-low',
] as const;

export type CategoryKey = (typeof CATEGORY_KEYS)[number];

/** agent key → 展示标签。 */
export const AGENT_LABELS: Record<string, string> = {
  sisyphus: 'Sisyphus',
  hephaestus: 'Hephaestus',
  oracle: 'Oracle',
  atlas: 'Atlas',
  metis: 'Metis',
  momus: 'Momus',
  'multimodal-looker': 'Multimodal Looker',
  'sisyphus-junior': 'Sisyphus Junior',
  prometheus: 'Prometheus',
  librarian: 'Librarian',
  explore: 'Explore',
};

/** category key → 展示标签。 */
export const CATEGORY_LABELS: Record<string, string> = {
  'visual-engineering': 'Visual Engineering',
  ultrabrain: 'Ultrabrain',
  deep: 'Deep',
  'unspecified-high': 'Unspecified High',
  artistry: 'Artistry',
  writing: 'Writing',
  quick: 'Quick',
  'unspecified-low': 'Unspecified Low',
};

/** 安全取标签，未知 key 退回原 key。 */
export function labelOf(map: Record<string, string>, key: string): string {
  return map[key] ?? key;
}
