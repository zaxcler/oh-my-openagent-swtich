/**
 * ModelRow 内部使用的数据形态。
 *
 * 与 `ModelEntry` 的区别：多一个 `id` 字段（model key），用于表单编辑。
 * 提交时由 EditPage 把数组 `[ModelRowValue]` 折叠成 `Record<id, ModelEntry>`。
 */
import type { Modalities, Modality } from '@/types';

const ALL_MODALITIES: Modality[] = ['text', 'image', 'audio', 'video', 'pdf'];

export interface ModelRowValue {
  id: string;
  name: string;
  group?: string;
  modalities?: Modalities;
}

export interface ModelRowProps {
  value: ModelRowValue;
  onChange: (entry: ModelRowValue) => void;
  onRemove: () => void;
}

function hasAnyModality(m: Modalities | undefined): boolean {
  return Boolean(m && (m.input.length > 0 || m.output.length > 0));
}

function toggleModality(arr: Modality[], value: Modality): Modality[] {
  return arr.includes(value) ? arr.filter((x) => x !== value) : [...arr, value];
}

/**
 * 单行 model 编辑器：id + name + 可选 group + 多模态(折叠) + 删除按钮。
 *
 * - id 必填（model 在 provider.models 字典里的 key）
 * - name 必填（展示名）
 * - group 可选（部分 provider 需要分组）
 * - modalities 可选（input/output 至少一项非空时折叠区默认展开；全空/未填则收起）
 *
 * 视觉风格：与 Layout 保持一致（daisyUI `input input-bordered` 配色 token）。
 */
export default function ModelRow({ value, onChange, onRemove }: ModelRowProps) {
  const hasModality = hasAnyModality(value.modalities);
  const inputList = value.modalities?.input ?? [];
  const outputList = value.modalities?.output ?? [];

  return (
    <div className="rounded-md border border-base-300 bg-base-100 p-3 space-y-2">
      <div className="grid grid-cols-[1fr_1fr_1fr_auto_auto] gap-2 items-end">
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
        <details
          className="dropdown dropdown-end"
          open={hasModality}
        >
          <summary
            className={`btn btn-sm ${
              hasModality ? 'btn-primary' : 'btn-ghost'
            } list-none cursor-pointer`}
            aria-label="编辑多模态"
            title="编辑多模态 (input / output)"
          >
            <span aria-hidden="true">🎨</span>
            <span className="ml-1 text-xs">多模态</span>
          </summary>
          <div className="dropdown-content z-20 mt-1 w-72 rounded-md border border-base-300 bg-base-100 p-3 shadow-lg space-y-3">
            <div>
              <div className="text-xs font-medium text-base-content/70 mb-1.5">
                Input（支持的输入）
              </div>
              <div className="flex flex-wrap gap-3">
                {ALL_MODALITIES.map((m) => (
                  <label key={`in-${m}`} className="label cursor-pointer gap-1.5 py-0">
                    <input
                      type="checkbox"
                      className="checkbox checkbox-xs"
                      checked={inputList.includes(m)}
                      onChange={() => {
                        const next = toggleModality(inputList, m);
                        onChange({
                          ...value,
                          modalities: {
                            input: next,
                            output: outputList,
                          },
                        });
                      }}
                      aria-label={`input ${m}`}
                    />
                    <span className="label-text text-xs">{m}</span>
                  </label>
                ))}
              </div>
            </div>
            <div>
              <div className="text-xs font-medium text-base-content/70 mb-1.5">
                Output（支持的输出）
              </div>
              <div className="flex flex-wrap gap-3">
                {ALL_MODALITIES.map((m) => (
                  <label key={`out-${m}`} className="label cursor-pointer gap-1.5 py-0">
                    <input
                      type="checkbox"
                      className="checkbox checkbox-xs"
                      checked={outputList.includes(m)}
                      onChange={() => {
                        const next = toggleModality(outputList, m);
                        onChange({
                          ...value,
                          modalities: {
                            input: inputList,
                            output: next,
                          },
                        });
                      }}
                      aria-label={`output ${m}`}
                    />
                    <span className="label-text text-xs">{m}</span>
                  </label>
                ))}
              </div>
            </div>
            {hasModality ? (
              <div className="flex items-center justify-between pt-1 border-t border-base-300/60">
                <span className="text-[10px] text-base-content/60">
                  已选 {inputList.length} in / {outputList.length} out
                </span>
                <button
                  type="button"
                  className="btn btn-ghost btn-xs"
                  onClick={() => onChange({ ...value, modalities: undefined })}
                  aria-label="清空多模态"
                  title="清空多模态(写入 JSON 时会省略该字段)"
                >
                  清空
                </button>
              </div>
            ) : null}
          </div>
        </details>
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
    </div>
  );
}
