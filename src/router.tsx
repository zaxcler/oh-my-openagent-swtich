/**
 * 应用路由配置。
 *
 * 路由表：
 * - `/`              列表页（默认入口）
 * - `/edit/:id?`     编辑页：`/edit/new` = 新建；`/edit/:id` = 编辑指定 config
 * - `/backups`       备份管理页（T14）
 * - `*`              兜底跳回列表
 *
 * Layout 作为 layout route（父级 element），其内部 <Outlet /> 渲染匹配的子路由页面。
 * 这样 <Link> 在 Layout 内部可以正常访问 Router Context。
 */
import { createBrowserRouter, Outlet } from 'react-router-dom';
import Layout from '@/components/Layout';
import ListPage from '@/pages/ListPage';
import EditPage from '@/pages/EditPage';
import BackupsPage from '@/pages/BackupsPage';

export const router = createBrowserRouter([
  {
    element: (
      <Layout>
        <Outlet />
      </Layout>
    ),
    children: [
      { path: '/', element: <ListPage /> },
      { path: '/edit/:id?', element: <EditPage /> },
      { path: '/backups', element: <BackupsPage /> },
      { path: '*', element: <ListPage /> },
    ],
  },
]);
