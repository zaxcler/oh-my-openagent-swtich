/**
 * 应用成功后的重启提示对话框。
 *
 * 内容：
 * - 标题「应用成功」+ 成功图标
 * - best-effort 进程检测结果提示（`isOpencodeRunning`，仅 UI 提示）
 * - 大字警示「需要重启 opencode 才生效」
 * - 备份文件路径列表
 * - 跨平台重启命令 + 「复制」按钮
 * - 「知道了」关闭按钮
 *
 * 视觉：daisyUI 语义类（与 ConfirmDialog 风格一致），原生 `<dialog>` 元素。
 */
import { useEffect, useRef, useState } from 'react';
import { getRestartCommand, isOpencodeRunning } from '@/lib/process-check';
import { showToast } from '@/store/toast';

export interface RestartPromptProps {
  /** 是否显示；变化时自动 open/close 底层 dialog */
  open: boolean;
  /** 本次 apply 备份的文件路径列表（来自 `ApplyResult.backup_files`） */
  backupFiles: string[];
  /** 关闭回调（点击知道了 / ESC / 遮罩） */
  onClose: () => void;
}

export default function RestartPrompt({
  open,
  backupFiles,
  onClose,
}: RestartPromptProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [running, setRunning] = useState<boolean | null>(null);
  const command = getRestartCommand();

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

  // 打开时做一次 best-effort 进程检测
  useEffect(() => {
    if (!open) {
      setRunning(null);
      return;
    }
    let cancelled = false;
    isOpencodeRunning()
      .then((result) => {
        if (!cancelled) setRunning(result);
      })
      .catch(() => {
        if (!cancelled) setRunning(null);
      });
    return () => {
      cancelled = true;
    };
  }, [open]);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(command);
      showToast('已复制重启命令', 'success');
    } catch (err) {
      showToast(`复制失败：${String(err)}`, 'error');
    }
  };

  return (
    <dialog
      ref={dialogRef}
      className="modal"
      onClose={onClose}
      onClick={(e) => {
        // 点击 backdrop 关闭；点击 modal-box 内不关闭
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="modal-box max-w-lg bg-base-100 border border-base-300"
        onClick={(e) => e.stopPropagation()}
      >
        {/* 标题：应用成功 */}
        <div className="flex items-center gap-2">
          <span className="text-success text-2xl" aria-hidden="true">
            ✓
          </span>
          <h3 className="text-lg font-semibold text-base-content">应用成功</h3>
        </div>

        {/* 进程状态提示（best-effort，null = 未完成/失败，不显示） */}
        {running === true ? (
          <div className="alert alert-warning mt-3 py-2 text-sm">
            <span aria-hidden="true">⚠</span>
            <span>
              opencode 似乎正在运行，重启前可能仍在使用旧配置。
            </span>
          </div>
        ) : null}
        {running === false ? (
          <div className="alert alert-info mt-3 py-2 text-sm">
            <span aria-hidden="true">ℹ</span>
            <span>未检测到 opencode 进程，下次启动将使用新配置。</span>
          </div>
        ) : null}

        {/* 大字提示：需要重启 */}
        <div className="mt-4 py-5 px-3 rounded-md bg-base-200 border border-base-300 text-center">
          <p className="text-2xl font-bold text-warning leading-tight">
            需要重启 opencode 才生效
          </p>
        </div>

        {/* 备份文件列表 */}
        {backupFiles.length > 0 ? (
          <div className="mt-4">
            <h4 className="text-sm font-medium text-base-content/80 mb-2">
              备份文件
            </h4>
            <ul className="text-xs text-base-content/70 space-y-1 max-h-40 overflow-auto bg-base-200 rounded p-2 border border-base-300">
              {backupFiles.map((p) => (
                <li key={p} className="font-mono break-all">
                  {p}
                </li>
              ))}
            </ul>
          </div>
        ) : null}

        {/* 重启命令 + 复制按钮 */}
        <div className="mt-4">
          <h4 className="text-sm font-medium text-base-content/80 mb-2">
            重启命令
          </h4>
          <div className="flex items-stretch gap-2">
            <code className="flex-1 text-xs font-mono bg-base-200 border border-base-300 rounded px-2 py-2 break-all select-all leading-relaxed">
              {command}
            </code>
            <button
              type="button"
              className="btn btn-primary btn-sm"
              onClick={() => void handleCopy()}
            >
              复制
            </button>
          </div>
        </div>

        {/* 关闭按钮 */}
        <div className="modal-action mt-6">
          <button
            type="button"
            className="btn btn-sm btn-ghost"
            onClick={onClose}
          >
            知道了
          </button>
        </div>
      </div>
      {/* backdrop 关闭按钮 */}
      <form method="dialog" className="modal-backdrop">
        <button type="submit" aria-label="关闭" onClick={onClose}>
          关闭
        </button>
      </form>
    </dialog>
  );
}
