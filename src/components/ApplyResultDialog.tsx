/**
 * 应用结果对话框 —— `apply_config` 成功后弹出。
 *
 * 职责：
 * - 告知用户应用成功（带时间戳）
 * - 列出本次备份的文件路径（可滚动 + 复制单条）
 * - 提示用户重启 opencode，并提供「复制重启命令」按钮
 *
 * 视觉：复用 ConfirmDialog 的 daisyUI `modal-box` 风格，保持应用内一致。
 * 复制走 `navigator.clipboard.writeText`（Tauri WebView 支持）。
 */
import { useEffect, useRef, useState } from 'react';
import { showToast } from '@/store/toast';
import type { ApplyResult } from '@/types';
import { getRestartCommand } from '@/lib/process-check';

export interface ApplyResultDialogProps {
  open: boolean;
  result: ApplyResult | null;
  onClose: () => void;
}

function formatAppliedAt(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString();
}

export default function ApplyResultDialog({
  open,
  result,
  onClose,
}: ApplyResultDialogProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) {
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  }, [open]);

  const handleCopyPath = async (path: string) => {
    try {
      await navigator.clipboard.writeText(path);
      showToast('已复制路径', 'success');
    } catch (err) {
      showToast(`复制失败：${String(err)}`, 'error');
    }
  };

  const [copiedCmd, setCopiedCmd] = useState(false);
  const handleCopyRestartCmd = async () => {
    const cmd = getRestartCommand();
    try {
      await navigator.clipboard.writeText(cmd);
      setCopiedCmd(true);
      showToast('已复制重启命令', 'success');
      window.setTimeout(() => setCopiedCmd(false), 1500);
    } catch (err) {
      showToast(`复制失败：${String(err)}`, 'error');
    }
  };

  return (
    <dialog
      ref={dialogRef}
      className="modal"
      onClose={onClose}
      onClick={onClose}
    >
      <div
        className="modal-box max-w-xl bg-base-100 border border-base-300"
        onClick={(e) => e.stopPropagation()}
      >
        <header className="flex items-start gap-3">
          <span
            aria-hidden="true"
            className="inline-flex h-9 w-9 items-center justify-center rounded-full bg-success/15 text-success text-lg font-bold"
          >
            ✓
          </span>
          <div className="flex-1 min-w-0">
            <h3 className="text-lg font-semibold text-base-content">
              应用成功
            </h3>
            {result ? (
              <p className="text-xs text-base-content/60 mt-1">
                {formatAppliedAt(result.applied_at)} ·{' '}
                opencode {result.opencode_updated ? '已更新' : '未变更'} ·{' '}
                omos {result.omos_updated ? '已更新' : '未变更'}
              </p>
            ) : null}
          </div>
        </header>

        {/* 备份文件列表 */}
        {result && result.backup_files.length > 0 ? (
          <section className="mt-5">
            <h4 className="text-sm font-medium text-base-content/80 mb-2">
              备份文件
            </h4>
            <ul className="max-h-48 overflow-auto rounded-md border border-base-300 divide-y divide-base-300">
              {result.backup_files.map((p) => (
                <li
                  key={p}
                  className="flex items-center gap-2 px-3 py-2 text-xs font-mono"
                >
                  <span className="flex-1 truncate" title={p}>
                    {p}
                  </span>
                  <button
                    type="button"
                    className="btn btn-ghost btn-xs"
                    onClick={() => void handleCopyPath(p)}
                    title="复制路径"
                  >
                    复制
                  </button>
                </li>
              ))}
            </ul>
          </section>
        ) : null}

        {/* 重启提示 */}
        <section className="mt-5 alert alert-info text-sm">
          <span aria-hidden="true">i</span>
          <div className="flex-1">
            <p className="font-medium">请重启 opencode 以加载新配置</p>
            <p className="text-xs opacity-80 mt-1">
              复制下方命令到终端执行，或手动退出后重新打开。
            </p>
            <div className="mt-2 flex items-center gap-2">
              <code className="flex-1 rounded bg-base-200 border border-base-300 px-2 py-1 text-xs font-mono overflow-x-auto">
                {getRestartCommand()}
              </code>
              <button
                type="button"
                className="btn btn-primary btn-sm"
                onClick={() => void handleCopyRestartCmd()}
              >
                {copiedCmd ? '已复制' : '复制重启命令'}
              </button>
            </div>
          </div>
        </section>

        <div className="modal-action mt-6">
          <button
            type="button"
            className="btn btn-primary btn-sm"
            onClick={onClose}
          >
            完成
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button type="submit" aria-label="关闭" onClick={onClose}>
          关闭
        </button>
      </form>
    </dialog>
  );
}
