/**
 * Tauri 应用启动 / 关闭辅助 —— T17 关键场景。
 *
 * tauri-driver 是 WebDriver 协议的服务端（默认 :4444），
 * Playwright 通过 webServer 配置启动它，每条 spec 自动得到独立的 WebDriver session。
 * 本文件仅保留一些「如果想手工 spawn」的备用入口。
 */
import { spawn, type ChildProcess } from 'node:child_process';

export function startTauriDev(): ChildProcess {
  return spawn('bun', ['run', 'tauri', 'dev'], {
    stdio: 'inherit',
    shell: true,
    env: process.env,
  });
}

export function stopTauriDev(proc: ChildProcess): void {
  if (proc.pid !== undefined) {
    proc.kill('SIGTERM');
  }
}
