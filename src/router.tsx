/**
 * 应用路由配置。
 *
 * 路由表：
 * - `/`              列表页（默认入口）
 * - `/edit/:id?`     编辑页：`/edit/new` = 新建；`/edit/:id` = 编辑指定 config
 * - `/backups`       备份管理页（T14）
 * - `*`              兜底跳回列表
 *
 * 路由定义集中在 `createBrowserRouter`（React Router v6 data router），
 * 后续 T15+ 如需 loader/action 可平滑叠加，不会改变入口签名。
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
