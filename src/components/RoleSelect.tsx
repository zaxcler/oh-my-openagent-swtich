export interface RoleSelectProps {
  /** 角色展示名（agent / category 标签）。 */
  label: string;
  /** 当前选中的 `omos/${modelId}` 字符串。允许空值（占位态）。 */
  value: string;
  /** 候选 option 列表，元素格式 `omos/${modelId}`。 */
  options: string[];
  /** 选项变更回调，参数为新的 `omos/${modelId}` 字符串。 */
  onChange: (value: string) => void;
}

/**
 * 单个 role (agent / category) 的模型下拉框。
 *
 * - 视觉风格：与 Layout 保持一致（daisyUI `select select-bordered` 配色 token）
 * - 当 `options` 为空时降级为禁用占位态："请先添加 model"
 * - 当 value 与 options 不匹配时（历史脏数据），不强行修正；
 *   用户选择一次后即恢复一致
 */
export default function RoleSelect({
  label,
  value,
  options,
  onChange,
}: RoleSelectProps) {
  const empty = options.length === 0;
  // 受控组件：浏览器只会显示 value 匹配的 option；这里做一层防御性归一化，
  // 避免在 value 为空字符串时浏览器把第一个 option 视觉上当成 value。
  const normalized = value && options.includes(value) ? value : '';

  return (
    <label className="form-control w-full">
      <div className="label py-1">
        <span className="label-text text-sm">{label}</span>
      </div>
      <select
        className="select select-bordered select-sm w-full"
        value={normalized}
        onChange={(e) => onChange(e.target.value)}
        disabled={empty}
        aria-label={label}
      >
        {empty ? (
          <option value="" disabled>
            请先添加 model
          </option>
        ) : (
          <>
            {normalized === '' && (
              <option value="" disabled>
                请选择
              </option>
            )}
            {options.map((opt) => (
              <option key={opt} value={opt}>
                {opt}
              </option>
            ))}
          </>
        )}
      </select>
    </label>
  );
}
