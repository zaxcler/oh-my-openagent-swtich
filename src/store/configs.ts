import { create } from 'zustand';
import type { ActiveStatus, ConfigMeta } from '@/types';

/**
 * 配置管理全局状态。
 *
 * 范围：
 * - 配置列表元信息（`configs`）
 * - 当前激活/漂移状态（`activeStatus`）
 * - 通用加载标志（`loading`）
 *
 * 详细 Config / Backup / ApplyResult 数据**不**入 store —— 它们是请求级数据，
 * 适合 React Query / SWR 之类的请求层。T12 引入时如无新方案，临时放进组件本地 state。
 */
interface ConfigsState {
  configs: ConfigMeta[];
  activeStatus: ActiveStatus | null;
  loading: boolean;

  setConfigs: (configs: ConfigMeta[]) => void;
  upsertConfig: (config: ConfigMeta) => void;
  removeConfig: (id: string) => void;
  setActiveStatus: (status: ActiveStatus) => void;
  setLoading: (loading: boolean) => void;
  reset: () => void;
}

export const useConfigsStore = create<ConfigsState>((set) => ({
  configs: [],
  activeStatus: null,
  loading: false,

  setConfigs: (configs) => set({ configs }),

  upsertConfig: (config) =>
    set((state) => {
      const idx = state.configs.findIndex((c) => c.id === config.id);
      if (idx === -1) {
        return { configs: [...state.configs, config] };
      }
      const next = state.configs.slice();
      next[idx] = config;
      return { configs: next };
    }),

  removeConfig: (id) =>
    set((state) => ({
      configs: state.configs.filter((c) => c.id !== id),
    })),

  setActiveStatus: (activeStatus) => set({ activeStatus }),
  setLoading: (loading) => set({ loading }),

  reset: () => set({ configs: [], activeStatus: null, loading: false }),
}));
