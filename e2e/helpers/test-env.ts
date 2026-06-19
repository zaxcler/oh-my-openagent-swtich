/**
 * E2E 测试环境路径与文件工具 —— T17 关键场景。
 *
 * 职责：
 * - 解析 playwright.config.ts 设置的 OMO_TEST_* 环境变量为绝对路径；
 * - 提供幂等的目录清理 / 文件落盘 / 读取工具。
 * - spec 严格走这里访问临时数据，绝不直接拼真实 `~/.config/opencode/`。
 */
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

function requireEnv(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(`env ${name} 未设置；playwright.config.ts 应在 spec 加载前完成初始化`);
  }
  return value;
}

export function testDataDir(): string {
  return requireEnv('TAURI_TEST_DATA_DIR');
}

export function testOpencodeDir(): string {
  return requireEnv('OMO_TEST_OPENCODE_DIR');
}

export function testConfigsDir(): string {
  return requireEnv('OMO_TEST_CONFIGS_DIR');
}

export function testBackupsDir(): string {
  return requireEnv('OMO_TEST_BACKUPS_DIR');
}

export function testActiveFile(): string {
  return requireEnv('OMO_TEST_ACTIVE_FILE');
}

export function testOpencodeJsoncPath(): string {
  return join(testOpencodeDir(), 'opencode.jsonc');
}

export function testOmosJsonPath(): string {
  return join(testOpencodeDir(), 'oh-my-openagent.json');
}

export function cleanTestData(): void {
  for (const dir of [testOpencodeDir(), testConfigsDir(), testBackupsDir()]) {
    if (existsSync(dir)) {
      rmSync(dir, { recursive: true, force: true });
    }
    mkdirSync(dir, { recursive: true });
  }
  const active = testActiveFile();
  if (existsSync(active)) {
    rmSync(active, { force: true });
  }
}

export function seedOpencodeJsonc(content: string): void {
  mkdirSync(testOpencodeDir(), { recursive: true });
  writeFileSync(testOpencodeJsoncPath(), content, 'utf8');
}

export function readOpencodeJsonc(): string | null {
  const path = testOpencodeJsoncPath();
  return existsSync(path) ? readFileSync(path, 'utf8') : null;
}

export function readOmosJson(): string | null {
  const path = testOmosJsonPath();
  return existsSync(path) ? readFileSync(path, 'utf8') : null;
}

export function readActiveJson(): string | null {
  const path = testActiveFile();
  return existsSync(path) ? readFileSync(path, 'utf8') : null;
}

export function listBackupFiles(): string[] {
  const dir = testBackupsDir();
  if (!existsSync(dir)) return [];
  return readdirSync(dir).filter((f) => f.endsWith('.json'));
}

