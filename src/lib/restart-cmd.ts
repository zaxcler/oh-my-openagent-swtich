/**
 * 平台相关：opencode 重启命令。
 *
 * - macOS / Linux  : `pkill -f opencode; opencode`
 * - Windows       : `taskkill /IM opencode.exe /F && start opencode`
 *
 * 通过 `navigator.platform` 判断（同步可用，无需 SSR 兼容——本项目仅在 Tauri WebView 中运行）。
 *
 * 任务约束：禁止在 UI 中使用 `window.confirm` / `alert` / 浏览器直接弹窗，
 * 但 shell 命令只是字符串拼接，不涉及 prompt。
 */
export function getRestartCommand(): string {
  const platform = navigator.platform.toLowerCase();
  if (platform.includes('mac') || platform.includes('linux')) {
    return 'pkill -f opencode; opencode';
  }
  return 'taskkill /IM opencode.exe /F && start opencode';
}
