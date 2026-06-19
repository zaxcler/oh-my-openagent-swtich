/**
 * best-effort opencode 进程检测。
 *
 * 设计原则：
 * - 仅用于 UI 提示（"opencode 可能还在运行"），**绝不**用于阻塞 apply_config
 * - 任何错误（plugin 未启用、权限不足、命令不存在）都返回 `false`
 * - 浏览器侧无法直接查询进程列表（WebView 安全模型），所以走 Tauri shell plugin
 *
 * 运行时依赖：
 * - 前端：已通过 `bun add @tauri-apps/plugin-shell` 装好
 * - 后端（src-tauri）：需在 `Cargo.toml` 添加 `tauri-plugin-shell = "2"`、
 *   在 `lib.rs` 注册 `.plugin(tauri_plugin_shell::init())`，
 *   并在 `capabilities/default.json` 加上 `"shell:default"` 权限。
 *   后端若未配置，exec() 会抛错被 catch 兜底，结果等价于"未运行"。
 */
export async function isOpencodeRunning(): Promise<boolean> {
  try {
    if (navigator.platform.toLowerCase().includes('win')) {
      // Windows: 无法从浏览器检测，返回 false（best-effort）
      return false;
    }
    // macOS/Linux: 通过 Tauri command 调 shell
    const { Command } = await import('@tauri-apps/plugin-shell');
    const result = await Command.create('pgrep', ['-f', 'opencode']).execute();
    return result.code === 0 && result.stdout.trim().length > 0;
  } catch {
    return false;
  }
}

/**
 * 根据平台返回"安全"的重启命令（仅供 UI 复制，不自动执行）。
 *
 * - **不**包含 `sudo`
 * - macOS / Linux：`pkill -f opencode` 后重新拉起
 * - Windows：`taskkill` + `start` 重新拉起
 *
 * 这只是给用户的"参考命令"，UI 层面只复制到剪贴板，由用户自己粘贴执行。
 */
export function getRestartCommand(): string {
  const platform = navigator.platform.toLowerCase();
  if (platform.includes('win')) {
    return 'taskkill /F /IM opencode.exe & start "" opencode';
  }
  // macOS / Linux —— 不使用 sudo
  return 'pkill -f opencode; opencode';
}
