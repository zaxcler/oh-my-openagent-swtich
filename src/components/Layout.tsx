import { Link } from 'react-router-dom';

/**
 * 顶层布局：仿 ccswitch 风格。
 *
 * 视觉规范：
 * - 顶部工具栏 52px，左对齐 logo + app 名称，右侧 "+ 新建" 圆形按钮
 * - 主体 padding 收紧，更紧凑
 * - 背景与窗口融为一体（macOS 标准）
 */
export default function Layout({ children }: { children: React.ReactNode }) {
  return (
    <div className="h-screen flex flex-col bg-base-100 text-base-content">
      <header className="h-[52px] px-4 flex items-center justify-between border-b border-base-300/60 shrink-0">
        <Link
          to="/"
          className="flex items-center gap-2 hover:opacity-80 transition-opacity"
        >
          <div className="w-7 h-7 rounded-md bg-gradient-to-br from-primary to-secondary flex items-center justify-center text-primary-content font-bold text-sm">
            ⇄
          </div>
          <span className="text-[15px] font-semibold tracking-tight">
            oh-my-openagent-switch
          </span>
        </Link>
        <div className="flex items-center gap-1">
          <Link
            to="/backups"
            className="btn btn-ghost btn-sm btn-circle"
            aria-label="备份管理"
            title="备份管理"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className="h-4 w-4"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4"
              />
            </svg>
          </Link>
          <Link
            to="/edit"
            className="btn btn-primary btn-sm btn-circle"
            aria-label="新建配置"
            title="新建配置"
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
                d="M12 4.5v15m7.5-7.5h-15"
              />
            </svg>
          </Link>
        </div>
      </header>
      <main className="flex-1 overflow-auto">{children}</main>
    </div>
  );
}
