/**
 * 配置列表页 —— T12 + T15。
 *
 * 职责：
 * - 加载配置列表 (`list_configs`) 和当前激活状态 (`get_active_status`)
 * - 渲染卡片列表，每张卡片显示 label / 更新时间 / 激活徽章
 * - 4 个操作：编辑 / 应用 / 导出 / 删除
 * - 空状态：通过 `<ImportFromOpencodeButton>` 一键导入
 * - 应用成功后：通过 `<ApplyButton>` 回调触发 `<RestartPrompt>` 并刷新状态
 *
 * 设计要点：
 * - 全局状态用 `useConfigsStore` (T11)；
 * - 局部 state 仅承载"瞬时 UI"（确认对话框、busy 状态、RestartPrompt 数据）；
 * - 所有 Tauri 调用集中在本页，`apiKey` 永不进入 DOM。
 * - T15 拆分：`ApplyButton` / `ImportFromOpencodeButton` / `RestartPrompt` 独立组件
 *   仅暴露必要 props；本页面负责编排（编排后立即 useEffect 拉新数据）。
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
  // T15: RestartPrompt 数据
  const [restartPrompt, setRestartPrompt] = useState<{
    open: boolean;
    backupFiles: string[];
  }>({ open: false, backupFiles: [] });

  // ----- 加载 + 刷新 -----

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

  /**
   * 应用成功后拉取最新状态（list + activeStatus）。
   * 抽出来供 ApplyButton 的 onApplied 回调复用。
   */
  const refreshStatus = useCallback(async () => {
    const list = await tauriInvoke<ConfigMeta[]>('list_configs');
    setConfigs(list);
    const status = await tauriInvoke<ActiveStatus>('get_active_status', {
      configs: list,
    });
    setActiveStatus(status);
  }, [setConfigs, setActiveStatus]);

  // ----- 操作：应用（编排层） -----

  const handleApplied = useCallback(
    async (result: ApplyResult) => {
      // 刷新列表 + activeStatus（其它徽章可能也会变）
      try {
        await refreshStatus();
      } catch (err) {
        showToast(`状态刷新失败：${String(err)}`, 'error');
      }
      // 弹重启提示（即使 backupFiles 为空也弹）
      setRestartPrompt({
        open: true,
        backupFiles: result.backup_files,
      });
    },
    [refreshStatus],
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

  // ----- 操作：从 opencode 一键导入（编排层） -----

  const handleImported = useCallback(
    (config: Config) => {
      // 把 meta 注入 store（store 只存 meta，full Config 通过 get_config 拿）
      upsertConfig({
        id: config.id,
        label: config.label,
        updated_at: config.updated_at,
      });
      showToast(`已导入：${config.label}`, 'success');
      // 跳转到编辑页，让用户继续填 agents / categories
      navigate(`/edit/${config.id}`);
    },
    [upsertConfig, navigate],
  );

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
              <ImportFromOpencodeButton onSuccess={handleImported} />
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
                      <ApplyButton
                        configId={meta.id}
                        label={meta.label}
                        onApplied={handleApplied}
                        busy={busy}
                      />
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

      {/* ----- 应用成功后的重启提示 ----- */}
      <RestartPrompt
        open={restartPrompt.open}
        backupFiles={restartPrompt.backupFiles}
        onClose={() => setRestartPrompt({ open: false, backupFiles: [] })}
      />
    </section>
  );
}
