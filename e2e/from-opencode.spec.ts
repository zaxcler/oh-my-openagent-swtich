/**
 * T17 E2E —— 关键场景 6：从 opencode 一键导入。
 *
 * 流程：准备含 provider.omos 的 opencode.jsonc → 列表空状态点
 * 「从 opencode 导入」 → 验证表单填充（label 自动生成、provider 字段从
 * opencode.jsonc 同步过来）。
 */
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { expect, test } from '@playwright/test';
import { cleanTestData, seedOpencodeJsonc, testDataDir } from './helpers/test-env';
import { maybeSkipOnUnsupportedPlatform } from './helpers/skip';

maybeSkipOnUnsupportedPlatform();

const FIXTURE_PATH = join(process.cwd(), 'e2e', 'fixtures', 'opencode.jsonc');

test.describe('from-opencode', () => {
  test.beforeEach(() => {
    process.env.TAURI_TEST_DATA_DIR = testDataDir();
    cleanTestData();
  });

  test.afterEach(() => {
    cleanTestData();
  });

  test('空状态点导入 → 跳转到编辑页且 provider 字段被填充', async ({ page }) => {
    const fixture = readFileSync(FIXTURE_PATH, 'utf8');
    seedOpencodeJsonc(fixture);

    await page.goto('/');
    await expect(page.getByRole('heading', { name: '还没有配置' })).toBeVisible();

    await page.getByRole('button', { name: '从 opencode 导入' }).click();

    // 跳转到 /edit/<id>，标题变为「编辑：Imported-...」
    await expect(page).toHaveURL(/\/edit\/[\w-]+$/);
    await expect(page.locator('h1', { hasText: /^编辑：/ })).toBeVisible();

    // provider 字段从 fixture 同步过来
    const expectedApiKey = 'sk-test-key';
    const expectedBaseUrl = 'http://localhost:3000/v1';
    const apiKeyInput = page.getByLabel('API Key *');
    const baseUrlInput = page.getByLabel('Base URL *');
    await expect(apiKeyInput).toHaveValue(expectedApiKey);
    await expect(baseUrlInput).toHaveValue(expectedBaseUrl);

    // model 行应至少有 1 行（test-model），id 已自动填好
    const modelId = page.getByLabel('model id');
    await expect(modelId).toHaveValue('test-model');
    await expect(page.getByLabel('model name')).toHaveValue('test-model');

    // label 是自动生成的 "Imported-..."，可能非空即可
    const labelInput = page.getByLabel('配置名称 *');
    await expect(labelInput).toHaveValue(/^Imported-/);
  });

  test('没有 provider.omos 时点击导入会触发错误 toast（不跳转）', async ({ page }) => {
    seedOpencodeJsonc(
      JSON.stringify({ plugin: ['x'], provider: { other: {} } }, null, 2),
    );

    await page.goto('/');
    await page.getByRole('button', { name: '从 opencode 导入' }).click();

    // 留在列表页：URL 不变
    await expect(page).toHaveURL(/\/$/);
    // 出现错误 toast
    await expect(page.locator('.alert, .toast').filter({ hasText: /未找到|导入失败/ })).toBeVisible({
      timeout: 5_000,
    });
  });
});
