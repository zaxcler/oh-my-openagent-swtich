/**
 * 备份管理页 —— T14。
 *
 * 职责：
 * - 加载 `list_backups`，按时间倒序展示
 * - 每条提供「恢复」与「删除」操作（均需 ConfirmDialog 二次确认）
 * - 不展示备份内容，只展示 meta（filename / original_path / created_at / size_bytes）
 *
 * 设计要点：
 * - 视觉与 ListPage 一致（daisyUI `card bg-base-200` + `border border-base-300`）
 * - 恢复/删除均通过 Tauri command（restore_backup / delete_backup）执行，
 *   前端**不**直接调 fs
 * - 操作完成 toast 提示
 */
import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { tauriInvoke } from '@/lib/tauri';
import type { BackupMeta } from '@/types';
import ConfirmDialog from '@/components/ConfirmDialog';
import { showToast } from '@/store/toast';

type PendingAction =
  | { kind: 'restore'; meta: BackupMeta }
  | { kind: 'delete'; meta: BackupMeta }
  | null;

function formatTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString();
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

export default function BackupsPage() {
  const [items, setItems] = useState<BackupMeta[]>([]);
  const [loading, setLoading] = useState(true);
  const [busyName, setBusyName] = useState<string | null>(null);
  const [pending, setPending] = useState<PendingAction>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await tauriInvoke<BackupMeta[]>('list_backups');
      setItems(list);
    } catch (err) {
      showToast(`加载备份失败：${String(err)}`, 'error');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const handleConfirm = useCallback(async () => {
    if (!pending) return;
    const { kind, meta } = pending;
    setBusyName(meta.filename);
    try {
      if (kind === 'restore') {
        await tauriInvoke('restore_backup', { filename: meta.filename });
        showToast(`已恢复：${meta.filename}`, 'success');
      } else {
        await tauriInvoke('delete_backup', { filename: meta.filename });
        showToast(`已删除：${meta.filename}`, 'success');
        setItems((prev) => prev.filter((m) => m.filename !== meta.filename));
      }
    } catch (err) {
      showToast(
        `${kind === 'restore' ? '恢复' : '删除'}失败：${String(err)}`,
        'error',
      );
    } finally {
      setBusyName(null);
      setPending(null);
    }
  }, [pending]);

  const dialogProps =
    pending === null
      ? null
      : pending.kind === 'restore'
        ? {
            open: true,
            title: '恢复备份？',
            message: `将从备份\n${pending.meta.filename}\n还原到原路径\n${pending.meta.original_path}\n此操作会覆盖当前文件，建议先确认是否需要导出当前配置。`,
            confirmLabel: '恢复',
            variant: 'primary' as const,
          }
        : {
            open: true,
            title: '删除备份？',
            message: `确认删除备份\n${pending.meta.filename}？\n此操作不可撤销。`,
            confirmLabel: '删除',
            variant: 'danger' as const,
          };

  return (
    <section className="max-w-3xl mx-auto py-4">
      {/* ----- 顶部工具栏 ----- */}
      <div className="flex items-center justify-between mb-4">
        <Link to="/" className="btn btn-ghost btn-sm" aria-label="返回列表">
          ← 返回列表
        </Link>
        <h1 className="text-lg font-semibold">备份管理</h1>
        <button
          type="button"
          className="btn btn-ghost btn-sm"
          onClick={() => void load()}
          disabled={loading}
          title="刷新"
          aria-label="刷新"
        >
          ↻
        </button>
      </div>

      {/* ----- 加载态 ----- */}
      {loading ? (
        <div className="flex justify-center py-16">
          <span className="loading loading-spinner loading-md text-primary" />
        </div>
      ) : null}

      {/* ----- 空态 ----- */}
      {!loading && items.length === 0 ? (
        <div className="card bg-base-200 border border-base-300 mt-8">
          <div className="card-body items-center text-center">
            <h2 className="card-title text-base-content/80">还没有备份</h2>
            <p className="text-sm text-base-content/60">
              在列表中点击「应用」生成配置时，系统会自动备份当前的
              <code className="mx-1 font-mono">opencode.jsonc</code>
              与
              <code className="mx-1 font-mono">oh-my-openagent.json</code>
              文件。
            </p>
          </div>
        </div>
      ) : null}

      {/* ----- 列表 ----- */}
      {items.length > 0 ? (
        <ul className="flex flex-col gap-3">
          {items.map((meta) => {
            const busy = busyName === meta.filename;
            return (
              <li
                key={meta.filename}
                className="card bg-base-200 border border-base-300"
              >
                <div className="card-body p-4">
                  <div className="flex items-start justify-between gap-4">
                    <div className="min-w-0 flex-1">
                      <h3
                        className="text-sm font-semibold font-mono truncate"
                        title={meta.filename}
                      >
                        {meta.filename}
                      </h3>
                      <p
                        className="text-xs text-base-content/60 mt-1 font-mono truncate"
                        title={meta.original_path}
                      >
                        原文件：{meta.original_path}
                      </p>
                      <p className="text-xs text-base-content/60 mt-1">
                        {formatTime(meta.created_at)} ·{' '}
                        {formatSize(meta.size_bytes)}
                      </p>
                    </div>
                    <div className="flex items-center gap-1 shrink-0">
                      <button
                        type="button"
                        className="btn btn-primary btn-sm"
                        onClick={() =>
                          setPending({ kind: 'restore', meta })
                        }
                        disabled={busy}
                        title="恢复到原路径"
                      >
                        {busy && pending?.meta.filename === meta.filename
                          ? '处理中…'
                          : '恢复'}
                      </button>
                      <button
                        type="button"
                        className="btn btn-ghost btn-sm text-error hover:bg-error/10"
                        onClick={() => setPending({ kind: 'delete', meta })}
                        disabled={busy}
                        title="删除此备份"
                      >
                        删除
                      </button>
                    </div>
                  </div>
                </div>
              </li>
            );
          })}
        </ul>
      ) : null}

      {/* ----- 确认对话框 ----- */}
      {dialogProps ? (
        <ConfirmDialog
          {...dialogProps}
          onConfirm={() => void handleConfirm()}
          onCancel={() => setPending(null)}
        />
      ) : null}
    </section>
  );
}
