import { Navigate, Route, Routes } from 'react-router-dom';
import ListPage from '@/pages/ListPage';
import EditPage from '@/pages/EditPage';

/**
 * 应用根路由。
 *
 * - `/`            配置列表
 * - `/edit`        新建配置
 * - `/edit/:id`    编辑指定配置
 * - 其它路径       跳转回列表
 */
export default function App() {
  return (
    <Routes>
      <Route path="/" element={<ListPage />} />
      <Route path="/edit" element={<EditPage />} />
      <Route path="/edit/:id" element={<EditPage />} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
