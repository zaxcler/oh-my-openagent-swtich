import { useParams } from 'react-router-dom';

/**
 * 配置编辑页占位。
 *
 * - `:id` 可选 —— 不存在时为"新建"模式
 * - T13 接入：react-hook-form + zod 校验 + ConfigPayload 表单
 */
export default function EditPage() {
  const { id } = useParams<{ id?: string }>();
  const mode = id ? '编辑' : '新建';

  return (
    <section className="max-w-3xl mx-auto py-8 text-center opacity-70">
      <h2 className="text-2xl font-semibold mb-2">
        {mode}配置 {id ? <code className="text-base-content/60">#{id}</code> : null}
      </h2>
      <p className="text-sm">TODO: 表单 + provider/agents/categories，T13 填充</p>
    </section>
  );
}
