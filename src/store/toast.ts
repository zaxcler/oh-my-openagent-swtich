/**
 * Toast 全局状态。
 *
 * 状态层与 UI 层分离：
 * - 本文件只导出 Zustand store 和便捷调用函数（**不导出 React 组件**），
 *   满足 `react-refresh/only-export-components` 规则，让 HMR 工作正常。
 * - 组件实现在 `@/components/Toast.tsx`。
 */
import { create } from 'zustand';

export type ToastVariant = 'success' | 'error' | 'warning';

export interface ToastItem {
  id: string;
  message: string;
  variant: ToastVariant;
}

interface ToastState {
  toasts: ToastItem[];
  show: (message: string, variant?: ToastVariant) => void;
  dismiss: (id: string) => void;
  clear: () => void;
}

/** 单调递增计数器，保证同一毫秒多次调用也互不冲突。 */
let counter = 0;
const nextId = () =>
  `toast-${Date.now().toString(36)}-${(counter++).toString(36)}`;

export const useToastStore = create<ToastState>((set) => ({
  toasts: [],
  show: (message, variant = 'success') => {
    const id = nextId();
    set((state) => ({ toasts: [...state.toasts, { id, message, variant }] }));
  },
  dismiss: (id) =>
    set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) })),
  clear: () => set({ toasts: [] }),
}));

/**
 * 业务层便捷 API —— 不直接 `useToastStore.getState().show`，
 * 让重构 store 实现时只改本文件。
 */
export function showToast(message: string, variant: ToastVariant = 'success'): void {
  useToastStore.getState().show(message, variant);
}
