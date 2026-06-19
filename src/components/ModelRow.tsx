/**
 * ModelRow 内部使用的数据形态。
 *
 * 与 `ModelEntry` 的区别：多一个 `id` 字段（model key），用于表单编辑。
 * 提交时由 EditPage 把数组 `[ModelRowValue]` 折叠成 `Record<id, ModelEntry>`。
 */
export interface ModelRowValue {
  id: string;
  name: string;
  group?: string;
}

export interface ModelRowProps {
  value: ModelRowValue;
  onChange: (entry: ModelRowValue) => void;
  onRemove: () => void;
}

/**
 * 单行 model 编辑器：id + name + 可选 group + 删除按钮。
 *
 * - id 必填（model 在 provider.models 字典里的 key）
 * - name 必填（展示名）
 * - group 可选（部分 provider 需要分组）
 *
 * 视觉风格：与 Layout 保持一致（daisyUI `input input-bordered` 配色 token）。
 */
export default function ModelRow({ value, onChange, onRemove }: ModelRowProps) {
  return (
    <div className="grid grid-cols-[1fr_1fr_1fr_auto] gap-2 items-end">
      <label className="form-control w-full">
        <div className="label py-1">
          <span className="label-text text-xs">Model ID *</span>
        </div>
        <input
          type="text"
          className="input input-bordered input-sm w-full"
          placeholder="gpt-4o"
          value={value.id}
          onChange={(e) => onChange({ ...value, id: e.target.value })}
          aria-label="model id"
          required
        />
      </label>
      <label className="form-control w-full">
        <div className="label py-1">
          <span className="label-text text-xs">Display Name *</span>
        </div>
        <input
          type="text"
          className="input input-bordered input-sm w-full"
          placeholder="GPT-4o"
          value={value.name}
          onChange={(e) => onChange({ ...value, name: e.target.value })}
          aria-label="model name"
          required
        />
      </label>
      <label className="form-control w-full">
        <div className="label py-1">
          <span className="label-text text-xs">Group</span>
        </div>
        <input
          type="text"
          className="input input-bordered input-sm w-full"
          placeholder="default"
          value={value.group ?? ''}
          onChange={(e) => onChange({ ...value, group: e.target.value })}
          aria-label="model group"
        />
      </label>
      <button
        type="button"
        className="btn btn-ghost btn-sm text-error"
        onClick={onRemove}
        aria-label="删除该 model"
        title="删除该 model"
      >
        ×
      </button>
    </div>
  );
}
