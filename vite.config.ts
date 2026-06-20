import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Tauri 在 dev 模式下连接到 1420 端口；这是 Vite + Tauri 2 的标准约定。
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  // Tauri 把 src/ 当作 frontendDist，所以 Vite 的 root 也指向 src/。
  root: 'src',
  base: './',

  plugins: [react(), tailwindcss()],

  // 防止 Vite 在解析过程中掩盖 Rust 错误。
  clearScreen: false,

  // 暴露给前端的 env 变量前缀（VITE_ / TAURI_ENV_）。
  envPrefix: ['VITE_', 'TAURI_ENV_'],

  server: {
    port: 1420,
    strictPort: true,
    host: host ?? false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 告诉 Vite 忽略 `src-tauri` 目录，避免 Rust 变更触发不必要的重载。
      ignored: ['**/src-tauri/**'],
    },
  },

  // 适配 Tauri 固定端口的预览服务。
  preview: {
    port: 1420,
    strictPort: true,
  },

  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },

  // 兼容旧浏览器，但避免 Tauri WebView2 加载时的 ESM 转 CJS 抖动。
  build: {
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    minify: process.env.TAURI_ENV_DEBUG ? false : 'esbuild',
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    outDir: path.resolve(__dirname, 'dist'),
  },
});
