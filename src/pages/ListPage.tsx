/**
 * 配置列表页 —— T12 核心页面。
 *
 * 职责：
 * - 加载配置列表 (`list_configs`) 和当前激活状态 (`get_active_status`)
 * - 渲染卡片列表，每张卡片显示 label / 更新时间 / 激活徽章
 * - 4 个操作：编辑 / 应用 / 导出 / 删除
 * - 空状态：从 opencode 一键导入
 *
 * 设计要点：
 * - 全局状态用 `useConfigsStore` (T11)；
 * - 局部 state 仅承载"瞬时 UI"（确认对话框、busy 状态）；
 * - 所有 Tauri 调用集中在本页，`apiKey` 永不进入 DOM。
 */
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { save as saveDialog } from '@tauri-apps/plugin-dialog';
import { tauriInvoke } from '@/lib/tauri';
import { useConfigsStore } from '@/store/configs';
import type { ActiveStatus, ConfigMeta } from '@/types';
import ConfirmDialog from '@/components/ConfirmDialog';
import { showToast } from '@/store/toast';

// ---------------------------------------------------------------------------
// 激活徽章
// ---------------------------------------------------------------------------

interface BadgeInfo {
  label: string;
  classes: string;
  icon: string;
}

const BADGE_SUCCESS: BadgeInfo = {
  label: '激活',
  icon: '✓',
  classes: 'badge badge-success',
};
const BADGE_WARNING: BadgeInfo = {
  label: '已偏离',
  icon: '⚠',
  classes: 'badge badge-warning',
};
const BADGE_NEUTRAL: BadgeInfo = {
  label: '未知',
  icon: '?',
  classes: 'badge badge-neutral',
};
const BADGE_ERROR: BadgeInfo = {
  label: '配置缺失',
  icon: '⚠',
  classes: 'badge badge-error',
};

/**
 * 将后端 `ActiveStatus` 映射为徽章信息。
 *
 * - `Active`     → 绿色 "✓ 激活"
 * - `Drifted`    → 黄色 "⚠ 已偏离"
 * - `Unknown`    → 灰色 "? 未知"
 * - `Orphan`     → 红色 "⚠ 配置缺失"
 */
function badgeFor(status: ActiveStatus | null, configId: string): BadgeInfo {
  if (!status) return BADGE_NEUTRAL;
  switch (status.type) {
    case 'Active':
      return status.config_id === configId ? BADGE_SUCCESS : BADGE_NEUTRAL;
    case 'Drifted':
      return status.config_id === configId ? BADGE_WARNING : BADGE_NEUTRAL;
    case 'Orphan':
      return status.reference_id === configId ? BADGE_ERROR : BADGE_NEUTRAL;
    case 'Unknown':
    default:
      return BADGE_NEUTRAL;
  }
}

// ---------------------------------------------------------------------------
// 时间格式化
// ---------------------------------------------------------------------------

/**
 * 格式化 ISO 时间戳为本地短字符串。
 * 解析失败时回退原字符串 —— 不抛错到 UI 层。
 */
function formatTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

// ---------------------------------------------------------------------------
// 主组件
// ---------------------------------------------------------------------------

export default function ListPage() {
  const navigate = useNavigate();

  // 全局 store
  const configs = useConfigsStore((s) => s.configs);
  const activeStatus = useConfigsStore((s) => s.activeStatus);
  const setConfigs = useConfigsStore((s) => s.setConfigs);
  const setActiveStatus = useConfigsStore((s) => s.setActiveStatus);
  const removeConfig = useConfigsStore((s) => s.removeConfig);
  const upsertConfig = useConfigsStore((s) => s.upsertConfig);

  // 瞬时 UI 状态
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<ConfigMeta | null>(null);
  const [importing, setImporting] = useState(false);

  // ----- 加载 -----

  const load = useCallback(async () => {
    setLoading(true);
    try {
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

  // ----- 操作：应用 -----

  const handleApply = useCallback(
    async (meta: ConfigMeta) => {
      setBusyId(meta.id);
      try {
        await tauriInvoke('apply_config', { id: meta.id });
        // 重新查询 active status（其它徽章可能也会变）
        const list = await tauriInvoke<ConfigMeta[]>('list_configs');
        setConfigs(list);
        const status = await tauriInvoke<ActiveStatus>('get_active_status', {
          configs: list,
        });
        setActiveStatus(status);
        showToast(`已应用：${meta.label}`, 'success');
      } catch (err) {
        showToast(`应用失败：${String(err)}`, 'error');
      } finally {
        setBusyId(null);
      }
    },
    [setConfigs, setActiveStatus],
  );

  // ----- 操作：导出 -----

  const handleExport = useCallback(async (meta: ConfigMeta) => {
    setBusyId(meta.id);
    try {
      const target = await saveDialog({
        title: '导出配置',
        defaultPath: `${meta.label || meta.id}.json`,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });
      if (!target) {
        // 用户取消
        return;
      }
      await tauriInvoke('export_config', { id: meta.id, target });
      showToast(`已导出到：${target}`, 'success');
    } catch (err) {
      showToast(`导出失败：${String(err)}`, 'error');
    } finally {
      setBusyId(null);
    }
  }, []);

  // ----- 操作：删除 -----

  const handleConfirmDelete = useCallback(async () => {
    if (!pendingDelete) return;
    const id = pendingDelete.id;
    setBusyId(id);
    try {
      await tauriInvoke('delete_config', { id });
      removeConfig(id);
      // active 状态可能变化（例如从 Active → Drifted/Unknown）
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

  // ----- 操作：从 opencode 一键导入 -----

  const handleImport = useCallback(async () => {
    setImporting(true);
    try {
      const created = await tauriInvoke<ConfigMeta | null>(
        'import_from_opencode',
      );
      if (!created) {
        showToast('当前 opencode.jsonc 没有可导入的 provider.omos 块', 'error');
        return;
      }
      // 后端命令返回完整 Config，但 store 只需要 meta；用同一命令回拉以保持单一数据源
      await load();
      upsertConfig(created);
      showToast(`已导入：${created.label}`, 'success');
    } catch (err) {
      showToast(`导入失败：${String(err)}`, 'error');
    } finally {
      setImporting(false);
    }
  }, [load, upsertConfig]);

  // ----- 渲染辅助 -----

  // 按 updated_at 降序
  const sortedConfigs = useMemo(
    () =>
      [...configs].sort((a, b) =>
        a.updated_at < b.updated_at ? 1 : a.updated_at > b.updated_at ? -1 : 0,
      ),
    [configs],
  );

  return (
    <section className="max-w-3xl mx-auto py-4">
      {/* ----- 空状态 ----- */}
      {!loading && sortedConfigs.length === 0 ? (
        <div className="card bg-base-200 border border-base-300 mt-8">
          <div className="card-body items-center text-center">
            <h2 className="card-title text-base-content/80">还没有配置</h2>
            <p className="text-sm text-base-content/60">
              点 <span className="font-mono">+</span> 新建第一个，或从当前
              opencode.jsonc 一键导入。
            </p>
            <div className="card-actions mt-2">
              <button
                type="button"
                className="btn btn-primary btn-sm"
                onClick={handleImport}
                disabled={importing}
              >
                {importing ? '导入中…' : '从 opencode 导入'}
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {/* ----- 加载态 ----- */}
      {loading ? (
        <div className="flex justify-center py-16">
          <span className="loading loading-spinner loading-md text-primary" />
        </div>
      ) : null}

      {/* ----- 列表 ----- */}
      {sortedConfigs.length > 0 ? (
        <ul className="flex flex-col gap-3">
          {sortedConfigs.map((meta) => {
            const badge = badgeFor(activeStatus, meta.id);
            const busy = busyId === meta.id;
            return (
              <li
                key={meta.id}
                className="card bg-base-200 border border-base-300"
              >
                <div className="card-body p-4">
                  <div className="flex items-start justify-between gap-4">
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2 flex-wrap">
                        <h3 className="text-base font-semibold truncate">
                          {meta.label || (
                            <span className="italic text-base-content/50">
                              未命名
                            </span>
                          )}
                        </h3>
                        <span
                          className={`${badge.classes} badge-sm`}
                          title={badge.label}
                        >
                          <span aria-hidden="true">{badge.icon}</span>
                          <span className="ml-1">{badge.label}</span>
                        </span>
                      </div>
                      <p className="text-xs text-base-content/60 mt-1">
                        更新于 {formatTime(meta.updated_at)}
                      </p>
                    </div>
                    <div className="flex items-center gap-1 shrink-0">
                      <button
                        type="button"
                        className="btn btn-ghost btn-sm"
                        onClick={() => navigate(`/edit/${meta.id}`)}
                        disabled={busy}
                        title="编辑"
                      >
                        编辑
                      </button>
                      <button
                        type="button"
                        className="btn btn-primary btn-sm"
                        onClick={() => void handleApply(meta)}
                        disabled={busy}
                        title="应用此配置到 opencode 环境"
                      >
                        {busy && busyId === meta.id ? '应用中…' : '应用'}
                      </button>
                      <button
                        type="button"
                        className="btn btn-ghost btn-sm"
                        onClick={() => void handleExport(meta)}
                        disabled={busy}
                        title="导出为 JSON 文件"
                      >
                        导出
                      </button>
                      <button
                        type="button"
                        className="btn btn-ghost btn-sm text-error hover:bg-error/10"
                        onClick={() => setPendingDelete(meta)}
                        disabled={busy}
                        title="删除配置"
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

      {/* ----- 删除确认 ----- */}
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
    </section>
  );
}
