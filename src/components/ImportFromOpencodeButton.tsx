/**
 * 「从 opencode 导入」按钮 —— 空状态使用。
 *
 * 行为：
 * - 点击 → 调 `import_from_opencode` (T9)；
 * - 返回 `Config | null`：
 *   - `Some(config)` → 触发 `onSuccess(config)` 回调（父级决定跳转 / 入库）
 *   - `None` → Toast 提示「未找到 provider.omos，请先在 opencode 中配置」
 * - 加载中显示 spinner，按钮 disabled
 *
 * 视觉：daisyUI 语义类（与 Layout / ConfirmDialog 一致）。
 */
import { useState } from 'react';
import { tauriInvoke } from '@/lib/tauri';
import { showToast } from '@/store/toast';
import type { Config } from '@/types';

export interface ImportFromOpencodeButtonProps {
  /**
   * 导入成功后的回调，父级通常用来：
   * - 跳转到 `/edit/${config.id}`
   * - 刷新列表 / activeStatus
   */
  onSuccess: (config: Config) => void;
  /** 自定义按钮文案，默认 "从 opencode 导入" */
  label?: string;
  /** 自定义 className（用于在空状态卡片中调整布局） */
  className?: string;
}

export default function ImportFromOpencodeButton({
  onSuccess,
  label = '从 opencode 导入',
  className = 'btn btn-primary btn-sm',
}: ImportFromOpencodeButtonProps) {
  const [busy, setBusy] = useState(false);

  const handleClick = async () => {
    if (busy) return;
    setBusy(true);
    try {
      const config = await tauriInvoke<Config | null>('import_from_opencode');
      if (!config) {
        showToast('未找到 provider.omos，请先在 opencode 中配置', 'error');
        return;
      }
      onSuccess(config);
    } catch (err) {
      showToast(`导入失败：${String(err)}`, 'error');
    } finally {
      setBusy(false);
    }
  };

  return (
    <button
      type="button"
      className={className}
      onClick={() => void handleClick()}
      disabled={busy}
      aria-busy={busy}
    >
      {busy ? (
        <>
          <span
            className="loading loading-spinner loading-xs"
            aria-hidden="true"
          />
          <span>导入中…</span>
        </>
      ) : (
        <span>{label}</span>
      )}
    </button>
  );
}
