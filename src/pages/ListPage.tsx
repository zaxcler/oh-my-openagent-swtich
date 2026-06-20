/**
 * 配置列表页 —— ccswitch 风格。
 *
 * 布局（每行单条配置）：
 * ┌────────────────────────────────────────────────────────────┐
 * │ [●]  名称                       激活徽章    [✎] [▶] [⤓] [🗑] │
 * │      API Base · 更新于 3 分钟前                            │
 * └────────────────────────────────────────────────────────────┘
 *
 * 整行 hover 高亮；左侧圆形 logo 自动用 provider 名首字母 + 渐变色。
 */
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { save as saveDialog } from '@tauri-apps/plugin-dialog';
import ApplyButton from '@/components/ApplyButton';
import ConfirmDialog from '@/components/ConfirmDialog';
import ImportFromOpencodeButton from '@/components/ImportFromOpencodeButton';
import RestartPrompt from '@/components/RestartPrompt';
import { tauriInvoke } from '@/lib/tauri';
import { useConfigsStore } from '@/store/configs';
import { showToast } from '@/store/toast';
import type { ActiveStatus, ApplyResult, Config, ConfigMeta } from '@/types';

// ---------------------------------------------------------------------------
// 激活徽章
// ---------------------------------------------------------------------------

interface BadgeStyle {
  label: string;
  className: string;
  dot: string;
}

const BADGE_ACTIVE: BadgeStyle = {
  label: '已激活',
  className: 'bg-success/15 text-success border-success/30',
  dot: 'bg-success',
};
const BADGE_DRIFTED: BadgeStyle = {
  label: '已偏离',
  className: 'bg-warning/15 text-warning border-warning/30',
  dot: 'bg-warning',
};
const BADGE_UNKNOWN: BadgeStyle = {
  label: '未激活',
  className: 'bg-base-content/10 text-base-content/60 border-base-content/10',
  dot: 'bg-base-content/40',
};
const BADGE_ORPHAN: BadgeStyle = {
  label: '配置缺失',
  className: 'bg-error/15 text-error border-error/30',
  dot: 'bg-error',
};

function badgeFor(status: ActiveStatus | null, configId: string): BadgeStyle {
  if (!status) return BADGE_UNKNOWN;
  switch (status.type) {
    case 'Active':
      return status.config_id === configId ? BADGE_ACTIVE : BADGE_UNKNOWN;
    case 'Drifted':
      return status.config_id === configId ? BADGE_DRIFTED : BADGE_UNKNOWN;
    case 'Orphan':
      return status.reference_id === configId ? BADGE_ORPHAN : BADGE_UNKNOWN;
    case 'Unknown':
    default:
      return BADGE_UNKNOWN;
  }
}

// ---------------------------------------------------------------------------
// 时间格式化
// ---------------------------------------------------------------------------

function formatTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

// ---------------------------------------------------------------------------
// Provider Logo：稳定 hash → 渐变 + 首字母
// ---------------------------------------------------------------------------

const GRADIENT_PAIRS: [string, string][] = [
  ['#f97316', '#ec4899'], // orange → pink
  ['#8b5cf6', '#3b82f6'], // violet → blue
  ['#10b981', '#06b6d4'], // emerald → cyan
  ['#f59e0b', '#ef4444'], // amber → red
  ['#6366f1', '#a855f7'], // indigo → purple
  ['#14b8a6', '#22c55e'], // teal → green
  ['#f43f5e', '#a855f7'], // rose → purple
  ['#3b82f6', '#06b6d4'], // blue → cyan
];

function hashString(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) {
    h = (h * 31 + s.charCodeAt(i)) | 0;
  }
  return Math.abs(h);
}

function ProviderLogo({
  name,
  label,
}: {
  name: string;
  label: string;
}) {
  const seed = hashString(name || label || '?');
  const [from, to] = GRADIENT_PAIRS[seed % GRADIENT_PAIRS.length];
  const initial = (name || label || '?').trim().charAt(0).toUpperCase();
  return (
    <div
      className="w-9 h-9 rounded-full flex items-center justify-center text-white font-bold text-sm shrink-0 shadow-sm"
      style={{
        background: `linear-gradient(135deg, ${from} 0%, ${to} 100%)`,
      }}
      aria-hidden="true"
    >
      {initial}
    </div>
  );
}

// ---------------------------------------------------------------------------
// 图标按钮
// ---------------------------------------------------------------------------

function IconButton({
  onClick,
  disabled,
  title,
  tone = 'default',
  children,
}: {
  onClick?: () => void;
  disabled?: boolean;
  title: string;
  tone?: 'default' | 'primary' | 'danger';
  children: React.ReactNode;
}) {
  const colorClass =
    tone === 'danger'
      ? 'hover:bg-error/10 hover:text-error'
      : tone === 'primary'
        ? 'hover:bg-primary/10 hover:text-primary'
        : 'hover:bg-base-content/10';
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      title={title}
      aria-label={title}
      className={`w-7 h-7 rounded-md flex items-center justify-center text-base-content/60 transition-colors disabled:opacity-40 disabled:cursor-not-allowed ${colorClass}`}
    >
      {children}
    </button>
  );
}

// ---------------------------------------------------------------------------
// 主组件
// ---------------------------------------------------------------------------

export default function ListPage() {
  const navigate = useNavigate();

  const configs = useConfigsStore((s) => s.configs);
  const activeStatus = useConfigsStore((s) => s.activeStatus);
  const setConfigs = useConfigsStore((s) => s.setConfigs);
  const setActiveStatus = useConfigsStore((s) => s.setActiveStatus);
  const removeConfig = useConfigsStore((s) => s.removeConfig);
  const upsertConfig = useConfigsStore((s) => s.upsertConfig);

  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<ConfigMeta | null>(null);
  const [restartPrompt, setRestartPrompt] = useState<{
    open: boolean;
    backupFiles: string[];
  }>({ open: false, backupFiles: [] });

  const load = useCallback(async () => {
    setLoading(true);
    try {
      // 启动时自动从 opencode.jsonc + oh-my-openagent.json 导入配置
      await tauriInvoke<Config | null>('auto_import_from_opencode');
      const list = await tauriInvoke<ConfigMeta[]>('list_configs');
      setConfigs(list);
      const status = await tauriInvoke<ActiveStatus>('get_active_status', {
        configs: list,
      });
      setActiveStatus(status);
    } catch (err) {
      showToast(`加载失败：${String(err)}`, 'error');
    } finally {
      setLoading(false);
    }
  }, [setConfigs, setActiveStatus]);

  useEffect(() => {
    void load();
  }, [load]);

  const refreshStatus = useCallback(async () => {
    const list = await tauriInvoke<ConfigMeta[]>('list_configs');
    setConfigs(list);
    const status = await tauriInvoke<ActiveStatus>('get_active_status', {
      configs: list,
    });
    setActiveStatus(status);
  }, [setConfigs, setActiveStatus]);

  const handleApplied = useCallback(
    async (result: ApplyResult) => {
      try {
        await refreshStatus();
      } catch (err) {
        showToast(`状态刷新失败：${String(err)}`, 'error');
      }
      setRestartPrompt({ open: true, backupFiles: result.backup_files });
    },
    [refreshStatus],
  );

  const handleExport = useCallback(async (meta: ConfigMeta) => {
    setBusyId(meta.id);
    try {
      const target = await saveDialog({
        title: '导出配置',
        defaultPath: `${meta.label || meta.id}.json`,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });
      if (!target) return;
      await tauriInvoke('export_config', { id: meta.id, target });
      showToast(`已导出到：${target}`, 'success');
    } catch (err) {
      showToast(`导出失败：${String(err)}`, 'error');
    } finally {
      setBusyId(null);
    }
  }, []);

  const handleDuplicate = useCallback(async (meta: ConfigMeta) => {
    setBusyId(meta.id);
    try {
      const created = await tauriInvoke<Config>('duplicate_config', { id: meta.id });
      const list = await tauriInvoke<ConfigMeta[]>('list_configs');
      setConfigs(list);
      const status = await tauriInvoke<ActiveStatus>('get_active_status', {
        configs: list,
      });
      setActiveStatus(status);
      showToast(`已复制为：${created.label}`, 'success');
    } catch (err) {
      showToast(`复制失败：${String(err)}`, 'error');
    } finally {
      setBusyId(null);
    }
  }, [setConfigs, setActiveStatus]);

  const handleConfirmDelete = useCallback(async () => {
    if (!pendingDelete) return;
    const id = pendingDelete.id;
    setBusyId(id);
    try {
      await tauriInvoke('delete_config', { id });
      removeConfig(id);
      const list = await tauriInvoke<ConfigMeta[]>('list_configs');
      setConfigs(list);
      const status = await tauriInvoke<ActiveStatus>('get_active_status', {
        configs: list,
      });
      setActiveStatus(status);
      showToast(`已删除：${pendingDelete.label}`, 'success');
      setPendingDelete(null);
    } catch (err) {
      showToast(`删除失败：${String(err)}`, 'error');
    } finally {
      setBusyId(null);
    }
  }, [pendingDelete, removeConfig, setConfigs, setActiveStatus]);

  const handleImported = useCallback(
    (config: Config) => {
      upsertConfig({
        id: config.id,
        label: config.label,
        updated_at: config.updated_at,
      });
      showToast(`已导入：${config.label}`, 'success');
      navigate(`/edit/${config.id}`);
    },
    [upsertConfig, navigate],
  );

  const sortedConfigs = useMemo(
    () =>
      [...configs].sort((a, b) =>
        a.updated_at < b.updated_at ? 1 : a.updated_at > b.updated_at ? -1 : 0,
      ),
    [configs],
  );

  return (
    <div className="max-w-3xl mx-auto px-4 py-4">
      {/* ----- 加载态 ----- */}
      {loading ? (
        <div className="flex justify-center py-20">
          <span className="loading loading-spinner loading-md text-primary" />
        </div>
      ) : null}

      {/* ----- 空状态 ----- */}
      {!loading && sortedConfigs.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 text-center">
          <div className="w-16 h-16 rounded-2xl bg-base-200 border border-base-300 flex items-center justify-center mb-4">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className="h-8 w-8 text-base-content/40"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={1.5}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M3.75 9.776c.112-.017.227-.026.344-.026h15.812c.117 0 .232.009.344.026m-16.5 0a2.25 2.25 0 00-1.883 2.542l.857 6a2.25 2.25 0 002.227 1.932H19.05a2.25 2.25 0 002.227-1.932l.857-6a2.25 2.25 0 00-1.883-2.542m-16.5 0V6A2.25 2.25 0 016 3.75h3.915a2.25 2.25 0 011.632.843l1.395 1.625a2.25 2.25 0 001.632.843H18A2.25 2.25 0 0120.25 9.776"
              />
            </svg>
          </div>
          <h2 className="text-base font-semibold text-base-content/80 mb-1">
            还没有配置
          </h2>
          <p className="text-sm text-base-content/50 mb-5 max-w-sm">
            点击右上角 <span className="font-mono mx-0.5">+</span> 新建，或从当前
            opencode.jsonc 一键导入
          </p>
          <ImportFromOpencodeButton onSuccess={handleImported} />
        </div>
      ) : null}

      {/* ----- 列表 ----- */}
      {sortedConfigs.length > 0 ? (
        <div className="space-y-1.5">
          {sortedConfigs.map((meta) => {
            const badge = badgeFor(activeStatus, meta.id);
            const busy = busyId === meta.id;
            return (
              <div
                key={meta.id}
                className="group flex items-center gap-3 px-3 py-2.5 rounded-lg hover:bg-base-200 transition-colors cursor-pointer"
                onClick={() => navigate(`/edit/${meta.id}`)}
              >
                <ProviderLogo
                  name={meta.label}
                  label={meta.label}
                />
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <h3 className="text-sm font-medium truncate">
                      {meta.label || (
                        <span className="italic text-base-content/40">
                          未命名
                        </span>
                      )}
                    </h3>
                    <span
                      className={`inline-flex items-center gap-1 px-1.5 py-0.5 text-[10px] font-medium border rounded-full ${badge.className}`}
                      title={badge.label}
                    >
                      <span
                        className={`w-1.5 h-1.5 rounded-full ${badge.dot}`}
                        aria-hidden="true"
                      />
                      {badge.label}
                    </span>
                  </div>
                  <p className="text-xs text-base-content/50 mt-0.5">
                    更新于 {formatTime(meta.updated_at)}
                  </p>
                </div>
                <div
                  className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity"
                  onClick={(e) => e.stopPropagation()}
                >
                  <IconButton
                    onClick={() => navigate(`/edit/${meta.id}`)}
                    disabled={busy}
                    title="编辑"
                  >
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      className="h-3.5 w-3.5"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                      strokeWidth={2}
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L10.582 16.07a4.5 4.5 0 01-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 011.13-1.897l8.932-8.931zm0 0L19.5 7.125"
                      />
                    </svg>
                  </IconButton>
                  <ApplyButton
                    configId={meta.id}
                    label={meta.label}
                    onApplied={handleApplied}
                    busy={busy}
                  />
                  <IconButton
                    onClick={() => void handleExport(meta)}
                    disabled={busy}
                    title="导出"
                  >
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      className="h-3.5 w-3.5"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                      strokeWidth={2}
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3"
                      />
                    </svg>
                  </IconButton>
                  <IconButton
                    onClick={() => void handleDuplicate(meta)}
                    disabled={busy}
                    title="复制"
                  >
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      className="h-3.5 w-3.5"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                      strokeWidth={2}
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        d="M15.75 17.25v3.375c0 .621-.504 1.125-1.125 1.125h-9.75a1.125 1.125 0 01-1.125-1.125V7.875c0-.621.504-1.125 1.125-1.125H6.75a9.06 9.06 0 011.5.124m7.5 10.376h3.375c.621 0 1.125-.504 1.125-1.125V11.25c0-4.46-3.243-8.161-7.5-8.876a9.06 9.06 0 00-1.5-.124H9.375c-.621 0-1.125.504-1.125 1.125v3.5m7.5 10.375H9.375a1.125 1.125 0 01-1.125-1.125v-9.25m12 6.625v-1.875a3.375 3.375 0 00-3.375-3.375h-1.5a1.125 1.125 0 01-1.125-1.125v-1.5a3.375 3.375 0 00-3.375-3.375H9.75"
                      />
                    </svg>
                  </IconButton>
                  <IconButton
                    onClick={() => setPendingDelete(meta)}
                    disabled={busy}
                    title="删除"
                    tone="danger"
                  >
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      className="h-3.5 w-3.5"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                      strokeWidth={2}
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0"
                      />
                    </svg>
                  </IconButton>
                </div>
              </div>
            );
          })}
        </div>
      ) : null}

      <ConfirmDialog
        open={pendingDelete !== null}
        title="删除配置？"
        message={
          pendingDelete
            ? `确认删除「${pendingDelete.label || '未命名'}」？\n此操作不可撤销。`
            : ''
        }
        confirmLabel="删除"
        variant="danger"
        onConfirm={() => void handleConfirmDelete()}
        onCancel={() => setPendingDelete(null)}
      />

      <RestartPrompt
        open={restartPrompt.open}
        backupFiles={restartPrompt.backupFiles}
        onClose={() => setRestartPrompt({ open: false, backupFiles: [] })}
      />
    </div>
  );
}
