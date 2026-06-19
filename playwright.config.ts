import { defineConfig, devices } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

/**
 * Playwright E2E 配置 —— T17 关键场景。
 *
 * 数据隔离：
 *   所有 OMO_TEST_* 路径在 Playwright 进程启动时即固定指向一个共享 tmpdir
 *   下各自的子目录。`webServer.env` 把它们透传给 tauri-driver，
 *   tauri-driver 启动 Tauri 应用时再透传下去 —— 真实 `~/.config/opencode/`
 *   永远不会被触碰。
 *
 * 单进程跑 (`workers: 1`)：单一 tauri-driver 实例 + 单一 Tauri 应用 session。
 *
 * 平台兼容：
 *   - macOS：tauri-driver 在 macOS 需要 CrabNebula Webdriver 或
 *     tauri-plugin-automation；本仓库未集成，整体 skip。spec 顶部还会再做一次
 *     显式 `test.skip` 兜底。
 *   - Linux / Windows：CI 需预装 webkit2gtk-driver / msedgedriver。
 */

const IS_DARWIN = process.platform === 'darwin';
const SKIP_ALL = IS_DARWIN && process.env.E2E_FORCE !== '1';

function makeTestDataDir(): string {
  const base = join(
    tmpdir(),
    `oh-my-openagent-switch-e2e-${process.pid}-${Date.now()}`,
  );
  mkdirSync(join(base, 'opencode'), { recursive: true });
  mkdirSync(join(base, 'configs'), { recursive: true });
  mkdirSync(join(base, 'backups'), { recursive: true });
  mkdirSync(join(base, 'app-root'), { recursive: true });
  return base;
}

const testDataDir = SKIP_ALL
  ? join(tmpdir(), 'omo-e2e-skipped')
  : makeTestDataDir();
const omoTestOpencodeDir = join(testDataDir, 'opencode');
const omoTestConfigsDir = join(testDataDir, 'configs');
const omoTestBackupsDir = join(testDataDir, 'backups');
const omoTestActiveFile = join(testDataDir, 'app-root', 'active.json');

process.env.TAURI_TEST_DATA_DIR = testDataDir;
process.env.OMO_TEST_OPENCODE_DIR = omoTestOpencodeDir;
process.env.OMO_TEST_CONFIGS_DIR = omoTestConfigsDir;
process.env.OMO_TEST_BACKUPS_DIR = omoTestBackupsDir;
process.env.OMO_TEST_ACTIVE_FILE = omoTestActiveFile;

export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: process.env.CI ? 'list' : 'html',
  timeout: 60_000,
  expect: { timeout: 10_000 },
  use: {
    baseURL: 'tauri://localhost',
    trace: 'retain-on-failure',
    video: 'retain-on-failure',
  },
  // 不在 config 层 testIgnore —— 那样 Playwright 会以 "0 tests" 退出码 1；
  // 平台跳过由每个 spec 顶部的 maybeSkipOnUnsupportedPlatform() 负责。
  webServer: SKIP_ALL
    ? undefined
    : {
        command: 'bunx tauri-driver --port 4444',
        url: 'http://127.0.0.1:4444/status',
        timeout: 60_000,
        reuseExistingServer: !process.env.CI,
        stdout: 'ignore',
        stderr: 'pipe',
        env: {
          OMO_TEST_OPENCODE_DIR: omoTestOpencodeDir,
          OMO_TEST_CONFIGS_DIR: omoTestConfigsDir,
          OMO_TEST_BACKUPS_DIR: omoTestBackupsDir,
          OMO_TEST_ACTIVE_FILE: omoTestActiveFile,
          TAURI_TEST_DATA_DIR: testDataDir,
        },
      },
  projects: [
    {
      name: 'tauri',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  metadata: {
    testDataDir,
    skipped: SKIP_ALL,
    reason: SKIP_ALL
      ? 'tauri-driver WKWebView not supported on macOS'
      : null,
  },
});
