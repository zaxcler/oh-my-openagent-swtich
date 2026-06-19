/**
 * 列表项「应用」按钮。
 *
 * 行为：
 * - 点击 → 弹出 ConfirmDialog 二次确认（防误触）
 * - 确认后调 `apply_config(configId)` (T9)
 * - 成功后：
 *   - Toast「已应用：xxx」
 *   - 触发 `onApplied(result)` 回调（父级用来弹 RestartPrompt + 刷新列表 / activeStatus）
 *
 * 设计原则：
 * - **不**在应用前阻塞等待 opencode 退出
 * - **不**在 RestartPrompt 出现前强制刷新（父级决定何时刷新）
 * - 自带 busy 状态，与外部传入的 `busy` prop 合并（避免多个按钮同时操作）
 *
 * 视觉：daisyUI 语义类（与 Layout / ConfirmDialog 一致）。
 */
import { useState } from 'react';
import ConfirmDialog from '@/components/ConfirmDialog';
import { tauriInvoke } from '@/lib/tauri';
import { showToast } from '@/store/toast';
import type { ApplyResult } from '@/types';

export interface ApplyButtonProps {
  /** 要应用的 config id */
  configId: string;
  /** 展示用 label（用于确认对话框文案） */
  label: string;
  /**
   * 应用成功后的回调，参数是 Rust 端 `apply_config` 返回的 `ApplyResult`。
   * 父级通常用来：弹 RestartPrompt + 刷新列表 / activeStatus。
   */
  onApplied: (result: ApplyResult) => void;
  /** 父级传入的 busy 状态（如"导出中"），用于禁用按钮 */
  busy?: boolean;
}

export default function ApplyButton({
  configId,
  label,
  onApplied,
  busy = false,
}: ApplyButtonProps) {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [applying, setApplying] = useState(false);

  const disabled = busy || applying;

  const handleApply = async () => {
    if (applying) return;
    setApplying(true);
    try {
      const result = await tauriInvoke<ApplyResult>('apply_config', {
        id: configId,
      });
      showToast(`已应用：${label}`, 'success');
      onApplied(result);
      setConfirmOpen(false);
    } catch (err) {
      showToast(`应用失败：${String(err)}`, 'error');
    } finally {
      setApplying(false);
    }
  };

  const handleCancel = () => {
    if (!applying) setConfirmOpen(false);
  };

  return (
    <>
      <button
        type="button"
        className="btn btn-primary btn-sm"
        onClick={() => setConfirmOpen(true)}
        disabled={disabled}
        title="应用此配置到 opencode 环境"
      >
        {applying ? '应用中…' : '应用'}
      </button>
      <ConfirmDialog
        open={confirmOpen}
        title="应用配置？"
        message={
          `确认将「${label || '未命名'}」应用到当前 opencode 环境？\n` +
          '应用前会自动备份 opencode.jsonc 和 oh-my-openagent.json。'
        }
        confirmLabel={applying ? '应用中…' : '应用'}
        cancelLabel="取消"
        variant="primary"
        onConfirm={() => void handleApply()}
        onCancel={handleCancel}
      />
    </>
  );
}
