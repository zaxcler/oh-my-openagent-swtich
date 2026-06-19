/**
 * T17 E2E —— 关键场景 5：激活状态徽章 4 种渲染。
 *
 * 4 个状态：
 *   - Active    → 绿色「✓ 激活」  (.badge-success)
 *   - Drifted   → 黄色「⚠ 已偏离」(.badge-warning)
 *   - Unknown   → 灰色「? 未知」  (.badge-neutral)
 *   - Orphan    → 红色「⚠ 配置缺失」(.badge-error)
 *
 * 准备方式：
 *   - Active  / Drifted：先创建 config，再调 `apply_config`；Drifted 场景再
 *     改一次 opencode.jsonc 制造指纹漂移。
 *   - Unknown  / Orphan：直接写 active.json 模拟。
 */
import { writeFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import {
  cleanTestData,
  readActiveJson,
  testActiveFile,
  testDataDir,
  testOpencodeJsoncPath,
} from './helpers/test-env';
import { maybeSkipOnUnsupportedPlatform } from './helpers/skip';

maybeSkipOnUnsupportedPlatform();

const LABEL = 'e2e-status';
const API_KEY = 'sk-status-key';
const BASE_URL = 'https://status.example.com/v1';
const MODEL_ID = 'status-model';
const MODEL_NAME = 'Status Model';

async function makeConfig(
  page: import('@playwright/test').Page,
  label: string,
): Promise<string> {
  await page.goto('/');
  await page.getByRole('link', { name: '新建配置' }).click();
  await page.getByLabel('配置名称 *').fill(label);
  await page.getByLabel('API Key *').fill(API_KEY);
  await page.getByLabel('Base URL *').fill(BASE_URL);
  await page.getByLabel('model id').fill(MODEL_ID);
  await page.getByLabel('model name').fill(MODEL_NAME);
  await page.getByRole('button', { name: '保存' }).click();
  await expect(page.locator('li', { hasText: label })).toBeVisible();

  return await page.evaluate(async (label) => {
    const mod = await import('/src/lib/tauri.ts');
    const list = await mod.tauriInvoke<{ id: string; label: string }[]>(
      'list_configs',
    );
    const found = list.find((c) => c.label === label);
    if (!found) throw new Error(`config ${label} not found`);
    return found.id;
  }, label);
}

async function applyConfig(
  page: import('@playwright/test').Page,
  label: string,
): Promise<void> {
  const card = page.locator('li', { hasText: label });
  await card.getByRole('button', { name: '应用' }).click();
  await page
    .locator('dialog.modal[open]')
    .getByRole('button', { name: /^应用$/ })
    .click();
  await expect(
    page.locator('dialog.modal[open]').getByText('应用成功'),
  ).toBeVisible({ timeout: 10_000 });
  await page
    .locator('dialog.modal[open]')
    .getByRole('button', { name: '知道了' })
    .click();
}

test.describe('active-status', () => {
  test.beforeEach(() => {
    process.env.TAURI_TEST_DATA_DIR = testDataDir();
    cleanTestData();
  });

  test.afterEach(() => {
    cleanTestData();
  });

  test('Active → 绿色「激活」徽章', async ({ page }) => {
    await makeConfig(page, LABEL);
    await applyConfig(page, LABEL);

    const card = page.locator('li', { hasText: LABEL });
    await expect(card.locator('.badge-success', { hasText: '激活' })).toBeVisible();
    expect(readActiveJson()).toBeTruthy();
  });

  test('Drifted → 黄色「已偏离」徽章', async ({ page }) => {
    await makeConfig(page, LABEL);
    await applyConfig(page, LABEL);

    // 改 opencode.jsonc 制造指纹漂移
    writeFileSync(
      testOpencodeJsoncPath(),
      JSON.stringify({ drifted: true, value: Math.random() }),
      'utf8',
    );

    // 重载列表
    await page.goto('/');
    const card = page.locator('li', { hasText: LABEL });
    await expect(card.locator('.badge-warning', { hasText: '已偏离' })).toBeVisible({
      timeout: 5_000,
    });
  });

  test('Unknown → 灰色「未知」徽章', async ({ page }) => {
    // 不写 active.json
    await makeConfig(page, LABEL);

    const card = page.locator('li', { hasText: LABEL });
    await expect(card.locator('.badge-neutral', { hasText: '未知' })).toBeVisible();
  });

  test('Orphan → 红色「配置缺失」徽章', async ({ page }) => {
    const id = await makeConfig(page, LABEL);

    // 写 active.json 引用一个并不存在的 config id
    const fake = {
      config_id: 'ghost-id-' + id,
      applied_at: new Date().toISOString(),
      fingerprints: { opencode: 'fp1', omos: 'fp2' },
    };
    writeFileSync(testActiveFile(), JSON.stringify(fake, null, 2), 'utf8');

    await page.goto('/');
    // 至少有一张卡显示「配置缺失」
    await expect(
      page.locator('.badge-error', { hasText: '配置缺失' }).first(),
    ).toBeVisible({ timeout: 5_000 });
  });
});
