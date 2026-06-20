import { useEffect, useState } from 'react';
import { Controller, useFieldArray, useForm, useWatch } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { z } from 'zod';

import ModelRow, { type ModelRowValue } from '@/components/ModelRow';
import RoleSelect from '@/components/RoleSelect';
import {
  AGENT_KEYS,
  AGENT_LABELS,
  CATEGORY_KEYS,
  CATEGORY_LABELS,
} from '@/lib/constants';
import { tauriInvoke } from '@/lib/tauri';
import { useConfigsStore } from '@/store/configs';
import { showToast } from '@/store/toast';
import type { Config, ConfigPayload, ModelEntry } from '@/types';

// ---------------------------------------------------------------------------
// 表单 schema
// ---------------------------------------------------------------------------

/**
 * 内部表单 schema。
 *
 * 与 `ConfigPayload` 的差异：
 * - `provider.models` 用数组（带 id）而不是 Record，便于 useFieldArray 操作
 * - `agents` / `categories` 用 `z.record(z.string())`（值 = `omos/${modelId}`）
 *
 * 提交时由 `toPayload()` 折叠成 Rust 端期望的形态。
 */
const schema = z.object({
  label: z.string().min(1, '配置名称必填'),
  provider: z.object({
    name: z
      .string()
      .min(1, '供应商名称必填')
      .regex(/^[A-Za-z][A-Za-z0-9_-]*$/, '供应商名称只能使用英文（字母/数字/下划线/连字符），且以字母开头'),
    npm: z.string().min(1, 'npm 必填'),
    options: z.object({
      api_key: z.string().min(1, 'apiKey 必填'),
      base_url: z.string().min(1, 'baseURL 必填'),
    }),
    models: z
      .array(
        z.object({
          id: z.string().min(1, 'model id 必填'),
          name: z.string().min(1, 'model name 必填'),
          group: z.string().optional(),
        }),
      )
      .min(1, '至少 1 个 model'),
  }),
  agents: z.record(z.string()),
  categories: z.record(z.string()),
});

type FormValues = z.infer<typeof schema>;

/** 一行空 model 模板（点击"添加 model"时复用）。 */
const EMPTY_MODEL: ModelRowValue = { id: '', name: '', group: '' };

/** 新建模式下的表单初始值。 */
const NEW_DEFAULTS: FormValues = {
  label: '',
  provider: {
    name: '',
    npm: '@ai-sdk/openai-compatible',
    options: { api_key: '', base_url: '' },
    models: [EMPTY_MODEL],
  },
  // 预填所有 role key 为空串，避开 zod `z.record(z.string())` 的 `Required` 校验。
  // 空串由 RoleSelect 视觉兜底为 options[0]，由 onSubmit 兜底成 omos/${firstId}。
  agents: Object.fromEntries(AGENT_KEYS.map((k) => [k, ''])),
  categories: Object.fromEntries(CATEGORY_KEYS.map((k) => [k, ''])),
};

// ---------------------------------------------------------------------------
// 工具函数
// ---------------------------------------------------------------------------

/**
 * 把表单数组形态的 models 折叠成 Rust 端期望的 `Record<id, ModelEntry>`。
 */
function toModelRecord(rows: ModelRowValue[]): Record<string, ModelEntry> {
  const out: Record<string, ModelEntry> = {};
  for (const row of rows) {
    if (!row.id) continue;
    out[row.id] = {
      name: row.name,
      ...(row.group ? { group: row.group } : {}),
    };
  }
  return out;
}

/**
 * 把 Rust 端的 `Record<id, ModelEntry>` 展开成表单数组形态。
 */
function toModelRows(record: Record<string, ModelEntry>): ModelRowValue[] {
  const rows = Object.entries(record).map(([id, entry]) => ({
    id,
    name: entry.name,
    group: entry.group ?? '',
  }));
  return rows.length > 0 ? rows : [EMPTY_MODEL];
}

/** 用 `omos/${modelId}` 格式化 option 字符串。 */
function toOption(id: string): string {
  return `omos/${id}`;
}

// ---------------------------------------------------------------------------
// 组件
// ---------------------------------------------------------------------------

/**
 * 配置编辑页：新建 / 编辑共用。
 *
 * - 路由：`/edit/new`（或 `/edit`）→ 新建；`/edit/:id` → 编辑
 * - 表单：react-hook-form + zod
 * - 提交：新建先调 `create_config` 再 `update_config`；编辑直接 `update_config`
 *
 * 注意：Rust 端 provider key 固定为 `omos`，前端**不**显示、不允许编辑。
 */
export default function EditPage() {
  const params = useParams<{ id?: string }>();
  const navigate = useNavigate();
  const upsertConfig = useConfigsStore((s) => s.upsertConfig);

  // `/edit/new` 也视为新建模式（与 `/edit` 等价）
  const editingId = params.id && params.id !== 'new' ? params.id : null;
  const isNew = editingId === null;

  const [loading, setLoading] = useState(!isNew);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [importingRoleJson, setImportingRoleJson] = useState(false);

  const {
    control,
    register,
    handleSubmit,
    reset,
    setValue,
    watch,
    formState: { errors },
  } = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: NEW_DEFAULTS,
    mode: 'onSubmit',
  });

  const { fields, append, remove } = useFieldArray({
    control,
    name: 'provider.models',
  });

  // 监听 models 用于生成 RoleSelect 的 options。角色默认值的同步由
  // RoleSelect 内部 fallback + onSubmit 兜底承担，避开 useEffect + getValues
  // 缓存在逐字符输入下失效的坑。
  const watchedModels = useWatch({ control, name: 'provider.models' });
  const modelOptions = (watchedModels ?? [])
    .filter((m): m is ModelRowValue => Boolean(m && m.id))
    .map((m) => toOption(m.id));

  // 编辑模式：加载已有配置到表单
  useEffect(() => {
    if (isNew) return;
    let cancelled = false;
    setLoading(true);
    setLoadError(null);
    tauriInvoke<Config>('get_config', { id: editingId })
      .then((config) => {
        if (cancelled) return;
        reset({
          label: config.label,
          provider: {
            name: config.payload.provider.name,
            npm: config.payload.provider.npm,
            options: { ...config.payload.provider.options },
            models: toModelRows(config.payload.provider.models),
          },
          agents: { ...config.payload.agents },
          categories: { ...config.payload.categories },
        });
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        const message = err instanceof Error ? err.message : String(err);
        setLoadError(message);
        console.error('[EditPage] get_config failed:', err);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [editingId, isNew, reset]);

  const handleImportRoleJson = async () => {
    if (importingRoleJson) return;
    setImportingRoleJson(true);
    try {
      const selected = await openDialog({
        title: '选择 oh-my-openagent 格式 JSON',
        multiple: false,
        filters: [{ name: 'JSON', extensions: ['json', 'jsonc'] }],
      });
      if (!selected) return;
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (!path) return;
      const result = await tauriInvoke<{
        agents: Record<string, string>;
        categories: Record<string, string>;
      }>('read_role_json_file', { path });
      let filled = 0;
      for (const key of AGENT_KEYS) {
        if (result.agents[key] !== undefined) {
          setValue(`agents.${key}`, result.agents[key], { shouldDirty: true });
          filled++;
        }
      }
      for (const key of CATEGORY_KEYS) {
        if (result.categories[key] !== undefined) {
          setValue(`categories.${key}`, result.categories[key], { shouldDirty: true });
          filled++;
        }
      }
      showToast(`已导入 ${filled} 项角色映射`, 'success');
    } catch (err) {
      showToast(`导入失败：${String(err)}`, 'error');
    } finally {
      setImportingRoleJson(false);
    }
  };

  const onInvalid = (errs: typeof errors) => {
    const list = Object.entries(errs)
      .map(([k, v]) => {
        const msg = v && 'message' in v ? (v as { message?: string }).message : JSON.stringify(v);
        return `${k}: ${msg}`;
      })
      .join('；');
    console.error('[EditPage] validation failed:', list, 'errs=', errs);
    showToast(`保存失败：${list || '请检查表单字段'}`, 'error');
  };

  const onSubmit = handleSubmit(async (data) => {
    if (submitting) return;
    setSubmitting(true);
    try {
      // 兜底：用户清空所有 role 时回填第一个 model（保证 Rust 端不会收到空字符串）
      const firstModelId = data.provider.models[0]?.id;
      const fallback = firstModelId ? toOption(firstModelId) : '';
      const safeAgents: Record<string, string> = {};
      for (const key of AGENT_KEYS) {
        safeAgents[key] = data.agents[key] || fallback;
      }
      const safeCategories: Record<string, string> = {};
      for (const key of CATEGORY_KEYS) {
        safeCategories[key] = data.categories[key] || fallback;
      }

      const payload: ConfigPayload = {
        label: data.label,
        provider: {
          name: data.provider.name,
          npm: data.provider.npm,
          options: { ...data.provider.options },
          models: toModelRecord(data.provider.models),
        },
        agents: safeAgents,
        categories: safeCategories,
      };

      if (isNew) {
        const created = await tauriInvoke<Config>('create_config', {
          label: data.label,
        });
        const updated = await tauriInvoke<Config>('update_config', {
          id: created.id,
          payload,
        });
        upsertConfig({
          id: updated.id,
          label: updated.label,
          updated_at: updated.updated_at,
        });
      } else {
        const updated = await tauriInvoke<Config>('update_config', {
          id: editingId,
          payload,
        });
        upsertConfig({
          id: updated.id,
          label: updated.label,
          updated_at: updated.updated_at,
        });
      }
      navigate('/');
    } catch (err) {
      console.error('[EditPage] save failed:', err);
      showToast(`保存失败：${String(err)}`, 'error');
    } finally {
      setSubmitting(false);
    }
  }, onInvalid);

  const titleLabel = watch('label');
  const title = isNew
    ? '新建配置'
    : `编辑：${titleLabel && titleLabel.length > 0 ? titleLabel : '未命名'}`;

  if (loading) {
    return (
      <div className="max-w-3xl mx-auto py-8 text-center">
        <span className="loading loading-spinner loading-md text-primary" />
        <p className="text-sm opacity-70 mt-3">加载中...</p>
      </div>
    );
  }

  if (loadError) {
    return (
      <div className="max-w-3xl mx-auto py-8 text-center">
        <h2 className="text-2xl font-semibold mb-2 text-error">加载失败</h2>
        <p className="text-sm opacity-70 mb-4">{loadError}</p>
        <Link to="/" className="btn btn-sm btn-ghost">
          返回列表
        </Link>
      </div>
    );
  }

  return (
    <div className="max-w-3xl mx-auto pb-32">
      {/* ----- 顶部工具栏：返回 + 标题 ----- */}
      <div className="sticky top-0 z-10 -mx-4 px-4 py-2 bg-base-100/85 backdrop-blur border-b border-base-300/60 flex items-center gap-3">
        <Link
          to="/"
          className="btn btn-ghost btn-sm btn-circle"
          aria-label="返回列表"
          title="返回列表"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            className="h-4 w-4"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2.5}
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M15.75 19.5L8.25 12l7.5-7.5"
            />
          </svg>
        </Link>
        <h1 className="text-base font-semibold truncate flex-1">{title}</h1>
      </div>

      <form onSubmit={onSubmit} className="space-y-6" noValidate>
        {/* ---------- 基础信息 ---------- */}
        <section className="card bg-base-200 p-4">
          <h2 className="text-lg font-medium mb-3">基础信息</h2>
          <label className="form-control w-full">
            <div className="label py-1">
              <span className="label-text">配置名称 *</span>
            </div>
            <input
              type="text"
              className={`input input-bordered w-full ${
                errors.label ? 'input-error' : ''
              }`}
              placeholder="my-config"
              {...register('label')}
            />
            {errors.label && (
              <div className="label py-1">
                <span className="label-text-alt text-error">
                  {errors.label.message}
                </span>
              </div>
            )}
          </label>
        </section>

        {/* ---------- 供应商设置 ---------- */}
        <section className="card bg-base-200 p-4">
          <h2 className="text-lg font-medium mb-3">
            供应商设置
            <span className="ml-2 text-xs opacity-50">
              （provider key 固定为 omos）
            </span>
          </h2>
          <div className="space-y-3">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <label className="form-control w-full">
                <div className="label py-1">
                  <span className="label-text">供应商名称 *（仅英文）</span>
                </div>
                <input
                  type="text"
                  className={`input input-bordered w-full ${
                    errors.provider?.name ? 'input-error' : ''
                  }`}
                  placeholder="openai"
                  {...register('provider.name')}
                />
                {errors.provider?.name && (
                  <div className="label py-1">
                    <span className="label-text-alt text-error">
                      {errors.provider.name.message}
                    </span>
                  </div>
                )}
              </label>
              <label className="form-control w-full">
                <div className="label py-1">
                  <span className="label-text">NPM 包</span>
                </div>
                <input
                  type="text"
                  className="input input-bordered w-full"
                  placeholder="@ai-sdk/openai-compatible"
                  {...register('provider.npm')}
                />
              </label>
            </div>
            <label className="form-control w-full">
              <div className="label py-1">
                <span className="label-text">API Key *</span>
              </div>
              <input
                type="password"
                className={`input input-bordered w-full ${
                  errors.provider?.options?.api_key ? 'input-error' : ''
                }`}
                placeholder="sk-..."
                autoComplete="off"
                {...register('provider.options.api_key')}
              />
              {errors.provider?.options?.api_key && (
                <div className="label py-1">
                  <span className="label-text-alt text-error">
                    {errors.provider.options.api_key.message}
                  </span>
                </div>
              )}
            </label>
            <label className="form-control w-full">
              <div className="label py-1">
                <span className="label-text">Base URL *</span>
              </div>
              <input
                type="text"
                className={`input input-bordered w-full ${
                  errors.provider?.options?.base_url ? 'input-error' : ''
                }`}
                placeholder="https://api.openai.com/v1"
                {...register('provider.options.base_url')}
              />
              {errors.provider?.options?.base_url && (
                <div className="label py-1">
                  <span className="label-text-alt text-error">
                    {errors.provider.options.base_url.message}
                  </span>
                </div>
              )}
            </label>

            <div>
              <div className="flex items-center justify-between mb-2">
                <h3 className="font-medium">Models *</h3>
                <button
                  type="button"
                  className="btn btn-sm btn-ghost"
                  onClick={() => append(EMPTY_MODEL)}
                >
                  + 添加 model
                </button>
              </div>
              <div className="space-y-2">
                {fields.map((field, index) => (
                  <Controller
                    key={field.id}
                    control={control}
                    name={`provider.models.${index}`}
                    render={({ field: f }) => (
                      <ModelRow
                        value={f.value as ModelRowValue}
                        onChange={f.onChange}
                        onRemove={() => remove(index)}
                      />
                    )}
                  />
                ))}
              </div>
              {errors.provider?.models?.message && (
                <div className="mt-1 text-sm text-error">
                  {errors.provider.models.message}
                </div>
              )}
            </div>
          </div>
        </section>

        {/* ---------- 角色模型设置 ---------- */}
        <section className="card bg-base-200 p-4">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-lg font-medium">角色模型设置</h2>
            <button
              type="button"
              className="btn btn-ghost btn-sm"
              onClick={() => void handleImportRoleJson()}
              disabled={importingRoleJson}
              title="从 oh-my-openagent.json 导入 agents/categories"
            >
              {importingRoleJson ? '导入中…' : '导入 JSON'}
            </button>
          </div>
          <h3 className="text-sm font-medium opacity-70 mb-2">Agents</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3 mb-6">
            {AGENT_KEYS.map((key) => (
              <Controller
                key={key}
                control={control}
                name={`agents.${key}`}
                render={({ field }) => (
                  <RoleSelect
                    label={AGENT_LABELS[key] ?? key}
                    value={field.value ?? ''}
                    options={modelOptions}
                    onChange={field.onChange}
                  />
                )}
              />
            ))}
          </div>
          <h3 className="text-sm font-medium opacity-70 mb-2">Categories</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {CATEGORY_KEYS.map((key) => (
              <Controller
                key={key}
                control={control}
                name={`categories.${key}`}
                render={({ field }) => (
                  <RoleSelect
                    label={CATEGORY_LABELS[key] ?? key}
                    value={field.value ?? ''}
                    options={modelOptions}
                    onChange={field.onChange}
                  />
                )}
              />
            ))}
          </div>
        </section>

        {/* ---------- 底部操作 ---------- */}
        <div className="sticky bottom-0 -mx-4 px-4 pt-3 pb-3 bg-base-100/90 backdrop-blur border-t border-base-300 flex justify-end gap-2">
          <Link to="/" className="btn btn-ghost btn-sm">
            取消
          </Link>
          <button
            type="submit"
            className="btn btn-primary btn-sm"
            disabled={submitting}
          >
            {submitting ? '保存中...' : '保存'}
          </button>
        </div>
      </form>
    </div>
  );
}
