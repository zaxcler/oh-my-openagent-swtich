import { spawn } from 'child_process';
import { waitFor } from '@playwright/test';

export async function startTauriApp() {
  const proc = spawn('bun', ['run', 'tauri', 'dev'], {
    stdio: 'inherit',
    shell: true,
  });
  await waitFor(() => proc.pid !== undefined, { timeout: 60000 });
  return proc;
}

export async function stopTauriApp(proc: ReturnType<typeof spawn>) {
  proc.kill('SIGTERM');
}
