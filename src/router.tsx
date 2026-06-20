/**
 * 应用路由配置。
 *
 * 路由表：
 * - `/`              列表页（默认入口）
 * - `/edit/:id?`     编辑页：`/edit/new` = 新建；`/edit/:id` = 编辑指定 config
 * - `/backups`       备份管理页（T14）
 * - `*`              兜底跳回列表
 *
 * Layout 在 main.tsx 中统一包裹（顶部栏 + 主区），路由只负责页面内容。
 */
import { createBrowserRouter } from 'react-router-dom';
import ListPage from '@/pages/ListPage';
import EditPage from '@/pages/EditPage';
import BackupsPage from '@/pages/BackupsPage';

export const router = createBrowserRouter([
  { path: '/', element: <ListPage /> },
  { path: '/edit/:id?', element: <EditPage /> },
  { path: '/backups', element: <BackupsPage /> },
  { path: '*', element: <ListPage /> },
]);
