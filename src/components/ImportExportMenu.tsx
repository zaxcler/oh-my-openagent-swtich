/**
 * 列表项的导入/导出按钮组。
 *
 * 行为：
 * - 导出：弹 `save()` 选保存路径 → 调 `export_config`
 * - 导入：弹 `open()` 选 JSON 文件 → 调 `import_config_file`（Rust 端会创建一个新 Config）
 *   导入成功后通过 `onImported` 通知父组件刷新列表
 *
 * 设计要点：
 * - 视觉与 ListPage 现有"导出"按钮一致：daisyUI `btn btn-ghost btn-sm`
 * - busy 状态内部维护（每个菜单独立），不污染父组件 store
 * - `disabled` 由父组件传入（与应用/删除等其它操作互斥）
 * - 错误/成功统一走 showToast
 */
import { useState } from 'react';
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
import { tauriInvoke } from '@/lib/tauri';
import { showToast } from '@/store/toast';
import type { Config, ConfigMeta } from '@/types';

export interface ImportExportMenuProps {
  /** 当前 config id（仅用于导出） */
  configId: string;
  /** 当前 config label（导出对话框默认文件名） */
  label: string;
  /** 父组件传入的总开关（如该 item 处于应用/删除 busy 态时禁用） */
  disabled?: boolean;
  /** 导入成功后回调，参数是新创建的 ConfigMeta（由 Rust 返回的 Config 折叠） */
  onImported?: (created: ConfigMeta) => void;
}

export default function ImportExportMenu({
  configId,
  label,
  disabled = false,
  onImported,
}: ImportExportMenuProps) {
  const [exporting, setExporting] = useState(false);
  const [importing, setImporting] = useState(false);

  const handleExport = async () => {
    if (exporting || importing) return;
    setExporting(true);
    try {
      const target = await saveDialog({
        title: '导出配置',
        defaultPath: `${label || configId}.json`,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });
      if (!target) return; // 用户取消
      await tauriInvoke('export_config', { id: configId, target });
      showToast(`已导出到：${target}`, 'success');
    } catch (err) {
      showToast(`导出失败：${String(err)}`, 'error');
    } finally {
      setExporting(false);
    }
  };

  const handleImport = async () => {
    if (exporting || importing) return;
    setImporting(true);
    try {
      const selected = await openDialog({
        title: '选择配置文件',
        multiple: false,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });
      if (!selected) return; // 用户取消
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (!path) return;
      const created = await tauriInvoke<Config>('import_config_file', { path });
      const meta: ConfigMeta = {
        id: created.id,
        label: created.label,
        updated_at: created.updated_at,
      };
      onImported?.(meta);
      showToast(`已导入：${created.label}`, 'success');
    } catch (err) {
      showToast(`导入失败：${String(err)}`, 'error');
    } finally {
      setImporting(false);
    }
  };

  const busy = exporting || importing;
  const blocked = disabled || busy;

  return (
    <div className="flex items-center gap-1">
      <button
        type="button"
        className="btn btn-ghost btn-sm"
        onClick={() => void handleExport()}
        disabled={blocked}
        title="导出为 JSON 文件"
      >
        {exporting ? '导出中…' : '导出'}
      </button>
      <button
        type="button"
        className="btn btn-ghost btn-sm"
        onClick={() => void handleImport()}
        disabled={blocked}
        title="从 JSON 文件导入（新建配置）"
      >
        {importing ? '导入中…' : '导入'}
      </button>
    </div>
  );
}
