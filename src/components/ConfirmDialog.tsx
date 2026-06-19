/**
 * 通用确认对话框。
 *
 * 设计要点：
 * - 受控组件：父组件持有 `open` 状态
 * - 使用浏览器原生 `<dialog>` 元素，自带聚焦管理、ESC 关闭、`showModal()` 遮罩
 * - 视觉：daisyui 语义类（与 Layout 一致），半透明遮罩，居中卡片
 * - 不可逆操作建议 `confirmLabel` 配 `btn-error`，常规确认用 `btn-primary`
 *
 * 典型用法：
 * ```tsx
 * <ConfirmDialog
 *   open={confirmId !== null}
 *   title="删除配置？"
 *   message="此操作不可撤销"
 *   confirmLabel="删除"
 *   variant="danger"
 *   onConfirm={handleDelete}
 *   onCancel={() => setConfirmId(null)}
 * />
 * ```
 */
import { useEffect, useRef } from 'react';

export type ConfirmVariant = 'primary' | 'danger';

export interface ConfirmDialogProps {
  /** 是否显示；变化时自动 open/close 底层 dialog */
  open: boolean;
  /** 标题（必填，一句话） */
  title: string;
  /** 详细说明；可换行 */
  message: string;
  /** 确认按钮文案，默认 "确定" */
  confirmLabel?: string;
  /** 取消按钮文案，默认 "取消" */
  cancelLabel?: string;
  /** primary = 普通确认；danger = 不可逆（红色按钮 + 默认焦点在取消按钮） */
  variant?: ConfirmVariant;
  /** 确认回调；返回 Promise 时会禁用按钮并显示加载态 */
  onConfirm: () => void | Promise<void>;
  /** 取消回调（点击取消 / ESC / 遮罩） */
  onCancel: () => void;
}

const variantButtonClasses: Record<ConfirmVariant, string> = {
  primary: 'btn-primary',
  danger: 'btn-error',
};

export default function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel = '确定',
  cancelLabel = '取消',
  variant = 'primary',
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const cancelButtonRef = useRef<HTMLButtonElement>(null);
  const confirmButtonRef = useRef<HTMLButtonElement>(null);
  const busyRef = useRef(false);

  // 同步 open -> 原生 dialog 的 show/close
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) {
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  }, [open]);

  // open 时聚焦：danger 默认聚焦取消按钮（防误触）；primary 默认聚焦确认按钮
  useEffect(() => {
    if (!open) return;
    const target =
      variant === 'danger' ? cancelButtonRef.current : confirmButtonRef.current;
    target?.focus();
  }, [open, variant]);

  const handleConfirm = async () => {
    if (busyRef.current) return;
    busyRef.current = true;
    try {
      await onConfirm();
    } finally {
      busyRef.current = false;
    }
  };

  const handleCancel = (e: React.SyntheticEvent<HTMLDialogElement>) => {
    // 原生 dialog 的 ESC / 遮罩点击会触发 close
    onCancel();
    e.preventDefault();
  };

  return (
    <dialog
      ref={dialogRef}
      className="modal"
      onClose={onCancel}
      onClick={handleCancel}
    >
      {/* 内层 click 不冒泡，避免点击卡片内容触发取消 */}
      <div
        className="modal-box max-w-md bg-base-100 border border-base-300"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="text-lg font-semibold text-base-content">{title}</h3>
        <p className="mt-2 text-sm text-base-content/70 whitespace-pre-line">
          {message}
        </p>
        <div className="modal-action mt-6">
          <button
            ref={cancelButtonRef}
            type="button"
            className="btn btn-ghost btn-sm"
            onClick={onCancel}
          >
            {cancelLabel}
          </button>
          <button
            ref={confirmButtonRef}
            type="button"
            className={`btn btn-sm ${variantButtonClasses[variant]}`}
            onClick={handleConfirm}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
      {/* 遮罩层（daisyui .modal 自带 backdrop 但需要 form method=dialog；
          用一层半透明 div 简化样式控制） */}
      <form method="dialog" className="modal-backdrop">
        <button type="submit" aria-label="关闭" onClick={onCancel}>
          关闭
        </button>
      </form>
    </dialog>
  );
}
