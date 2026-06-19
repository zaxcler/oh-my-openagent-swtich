/**
 * T17 E2E —— 关键场景 3：备份 / 恢复。
 *
 * 流程：先应用一次产生备份 → 进入 /backups → 看到备份项 → 点恢复 →
 * 验证原文件被回退到备份时的内容。
 */
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import {
  cleanTestData,
  listBackupFiles,
  readOpencodeJsonc,
  seedOpencodeJsonc,
  testDataDir,
  testOpencodeJsoncPath,
} from './helpers/test-env';
import { maybeSkipOnUnsupportedPlatform } from './helpers/skip';

maybeSkipOnUnsupportedPlatform();

const LABEL = 'e2e-backup';
const API_KEY = 'sk-backup-key';
const BASE_URL = 'https://backup.example.com/v1';
const MODEL_ID = 'backup-model';
const MODEL_NAME = 'Backup Model';
const ORIGINAL_CONTENT = '{"version":1,"user":"alice","note":"original"}';

async function createAndApply(
  page: import('@playwright/test').Page,
  label: string,
): Promise<void> {
  await page.goto('/');
  await page.getByRole('link', { name: '新建配置' }).click();
  await page.getByLabel('配置名称 *').fill(label);
  await page.getByLabel('API Key *').fill(API_KEY);
  await page.getByLabel('Base URL *').fill(BASE_URL);
  await page.getByLabel('model id').fill(MODEL_ID);
  await page.getByLabel('model name').fill(MODEL_NAME);
  await page.getByRole('button', { name: '保存' }).click();

  const card = page.locator('li', { hasText: label });
  await card.getByRole('button', { name: '应用' }).click();
  await page.locator('dialog.modal[open]').getByRole('button', { name: /^应用$/ }).click();
  await expect(
    page.locator('dialog.modal[open]').getByText('应用成功'),
  ).toBeVisible({ timeout: 10_000 });
  // 关闭 RestartPrompt 以免干扰后续导航
  await page.locator('dialog.modal[open]').getByRole('button', { name: '知道了' }).click();
}

test.describe('backup-restore', () => {
  test.beforeEach(() => {
    process.env.TAURI_TEST_DATA_DIR = testDataDir();
    cleanTestData();
  });

  test.afterEach(() => {
    cleanTestData();
  });

  test('应用后进入 /backups 可看到备份；恢复后原文件回滚', async ({ page }) => {
    seedOpencodeJsonc(ORIGINAL_CONTENT);
    expect(readFileSync(testOpencodeJsoncPath(), 'utf8')).toBe(ORIGINAL_CONTENT);

    await createAndApply(page, LABEL);

    // 备份目录应至少有一个 json
    await expect.poll(() => listBackupFiles().length, { timeout: 5000 }).toBeGreaterThan(0);
    const backupFiles = listBackupFiles();
    expect(backupFiles.length).toBeGreaterThan(0);

    // 现在手动把 opencode.jsonc 改成另一份内容，方便后续验证恢复
    const polluted = '{"version":99,"user":"mallory"}';
    writeFileSync(testOpencodeJsoncPath(), polluted, 'utf8');
    expect(readOpencodeJsonc()).toBe(polluted);

    // 进入备份页
    await page.goto('/backups');
    await expect(page.getByRole('heading', { name: '备份管理' })).toBeVisible();

    // 列表里至少一个备份
    const items = page.locator('ul > li');
    await expect(items.first()).toBeVisible();

    // 点恢复 → 确认 → 成功
    await items.first().getByRole('button', { name: '恢复' }).click();
    await expect(page.locator('dialog.modal[open]')).toBeVisible();
    await page.locator('dialog.modal[open]').getByRole('button', { name: '恢复' }).click();

    // 等待 opencode.jsonc 被原子写回
    await expect.poll(() => readOpencodeJsonc(), { timeout: 5000 }).toBe(ORIGINAL_CONTENT);

    // 备份元信息列表依然存在（恢复不删备份）
    expect(listBackupFiles().length).toBeGreaterThan(0);
    expect(existsSync(testOpencodeJsoncPath())).toBe(true);
  });
});
