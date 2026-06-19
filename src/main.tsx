import React from 'react';
import ReactDOM from 'react-dom/client';
import { RouterProvider } from 'react-router-dom';
import Layout from '@/components/Layout';
import { ToastContainer } from '@/components/Toast';
import { router } from '@/router';
import '@/index.css';

const rootEl = document.getElementById('root');
if (!rootEl) {
  throw new Error('Root element #root not found in index.html');
}

ReactDOM.createRoot(rootEl).render(
  <React.StrictMode>
    <Layout>
      <RouterProvider router={router} />
    </Layout>
    <ToastContainer />
  </React.StrictMode>,
);
