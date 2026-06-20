/**
 * 全局 Toast 通知 —— UI 层。
 *
 * 架构：
 * - 状态层：`@/store/toast`（Zustand store + showToast 便捷调用）
 * - UI 层：本文件（`ToastContainer` + `Toast`，固定右上角 + 3s 自动消失）
 *
 * 集成方式：
 * - `main.tsx` 在最外层挂载 `<ToastContainer />` 一次
 * - 业务侧 `import { showToast } from '@/store/toast'`
 *
 * 视觉：daisyui 语义类（与项目其它组件一致），无需额外 CSS。
 */
import { useEffect } from 'react';
import { useToastStore } from '@/store/toast';
import type { ToastVariant, ToastItem } from '@/store/toast';

const AUTO_DISMISS_MS = 3000;

const variantClasses: Record<ToastVariant, string> = {
  success: 'alert-success',
  error: 'alert-error',
  warning: 'alert-warning',
};

const variantIcon: Record<ToastVariant, string> = {
  success: '✓',
  error: '✕',
  warning: '!',
};

interface ToastProps {
  item: ToastItem;
}

/**
 * 单条 Toast 项 —— 含自动消失计时器和手动关闭。
 */
function Toast({ item }: ToastProps) {
  const dismiss = useToastStore((s) => s.dismiss);

  useEffect(() => {
    const timer = window.setTimeout(() => dismiss(item.id), AUTO_DISMISS_MS);
    return () => window.clearTimeout(timer);
  }, [item.id, dismiss]);

  return (
    <div
      role="status"
      aria-live="polite"
      className={`alert ${variantClasses[item.variant]} shadow-lg pr-2 min-w-[16rem] max-w-md`}
    >
      <span className="text-base font-semibold" aria-hidden="true">
        {variantIcon[item.variant]}
      </span>
      <span className="flex-1 text-sm">{item.message}</span>
      <button
        type="button"
        className="btn btn-ghost btn-xs"
        onClick={() => dismiss(item.id)}
        aria-label="关闭通知"
      >
        ✕
      </button>
    </div>
  );
}

/**
 * 挂载在应用根部的 Toast 容器。
 *
 * - 固定在右上角 (top-4 right-4)
 * - 使用 flex column 堆叠多条
 * - `pointer-events-none` 让背景可点击，仅 toast 自身可交互
 */
export function ToastContainer() {
  const toasts = useToastStore((s) => s.toasts);

  if (toasts.length === 0) return null;

  return (
    <div
      className="fixed top-4 right-4 z-50 flex flex-col gap-2 pointer-events-none"
      aria-label="通知"
    >
      {toasts.map((t) => (
        <div key={t.id} className="pointer-events-auto">
          <Toast item={t} />
        </div>
      ))}
    </div>
  );
}
