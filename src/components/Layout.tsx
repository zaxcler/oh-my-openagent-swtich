import { Link } from 'react-router-dom';

interface LayoutProps {
  children: React.ReactNode;
}

/**
 * 顶层布局：固定高度、顶部导航、内容区可滚动。
 *
 * 设计约束：
 * - 顶部栏 56px (`h-14`)，底部不带任何粘性 footer
 * - 主区域 `flex-1 overflow-auto` —— 内部页面自己负责滚动
 * - "+" 按钮：当前阶段只渲染占位，T13 接入"新建配置"流程
 */
export default function Layout({ children }: LayoutProps) {
  return (
    <div className="h-screen flex flex-col bg-base-100 text-base-content">
      <header className="h-14 border-b border-base-300 flex items-center justify-between px-4 shrink-0">
        <Link to="/" className="flex items-center gap-2">
          <span className="text-lg font-bold tracking-tight">
            OH-MY-OPENAGENT-SWITCH
          </span>
        </Link>
        <Link
          to="/edit"
          className="btn btn-primary btn-sm"
          aria-label="新建配置"
          title="新建配置"
        >
          +
        </Link>
      </header>
      <main className="flex-1 overflow-auto p-4">{children}</main>
    </div>
  );
}
