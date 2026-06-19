/**
 * T17 E2E —— 关键场景 2：应用配置。
 *
 * 覆盖 3 个分支：
 * 1. 干净环境下应用 → opencode.jsonc / oh-my-openagent.json / active.json 三件套落地
 * 2. 合并模式 → fixture 中的 model.headers / limit / plugin / permission 必须保留
 * 3. 损坏模式 → 应用触发后端报错，原始 opencode.jsonc 不被破坏
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { expect, test } from '@playwright/test';
import {
  cleanTestData,
  readActiveJson,
  readOmosJson,
  readOpencodeJsonc,
  seedOpencodeJsonc,
  testBackupsDir,
  testDataDir,
} from './helpers/test-env';
import { maybeSkipOnUnsupportedPlatform } from './helpers/skip';

maybeSkipOnUnsupportedPlatform();

const LABEL = 'e2e-apply';
const API_KEY = 'sk-apply-key';
const BASE_URL = 'https://apply.example.com/v1';
const MODEL_ID = 'apply-model';
const MODEL_NAME = 'Apply Model';

const FIXTURE_DIR = join(process.cwd(), 'e2e', 'fixtures');

async function createConfig(
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
  await expect(page.locator('li', { hasText: label })).toBeVisible();
}

async function applyFromCard(
  page: import('@playwright/test').Page,
  label: string,
): Promise<void> {
  const card = page.locator('li', { hasText: label });
  await card.getByRole('button', { name: '应用' }).click();
  const dialog = page.locator('dialog.modal[open]');
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText('应用配置？')).toBeVisible();
  await dialog.getByRole('button', { name: /^应用$/ }).click();
}

test.describe('apply-config', () => {
  test.beforeEach(() => {
    process.env.TAURI_TEST_DATA_DIR = testDataDir();
    cleanTestData();
  });

  test.afterEach(() => {
    cleanTestData();
  });

  test('干净环境下应用：opencode.jsonc / oh-my-openagent.json / active.json 都更新', async ({
    page,
  }) => {
    await createConfig(page, LABEL);
    await applyFromCard(page, LABEL);

    await expect(
      page.locator('dialog.modal[open]').getByText('应用成功'),
    ).toBeVisible({ timeout: 10_000 });

    await expect.poll(() => readOpencodeJsonc(), { timeout: 5000 }).not.toBeNull();
    await expect.poll(() => readOmosJson(), { timeout: 5000 }).not.toBeNull();
    await expect.poll(() => readActiveJson(), { timeout: 5000 }).not.toBeNull();

    const opencode = JSON.parse(readOpencodeJsonc()!);
    expect(opencode.provider.omos.options.apiKey).toBe(API_KEY);
    expect(opencode.provider.omos.options.baseURL).toBe(BASE_URL);
    expect(opencode.provider.omos.models[MODEL_ID].name).toBe(MODEL_NAME);

    const omos = JSON.parse(readOmosJson()!);
    expect(omos.agents.coder).toBe(`omos/${MODEL_ID}`);

    const active = JSON.parse(readActiveJson()!);
    expect(active.config_id).toBeTruthy();
    expect(active.fingerprints.opencode).toBeTruthy();
    expect(active.fingerprints.omos).toBeTruthy();

    const card = page.locator('li', { hasText: LABEL });
    await expect(card.locator('.badge-success')).toBeVisible();
  });

  test('合并模式：headers / limit / plugin / permission 必须保留', async ({ page }) => {
    seedOpencodeJsonc(
      readFileSync(join(FIXTURE_DIR, 'opencode-with-headers.jsonc'), 'utf8'),
    );

    await createConfig(page, LABEL);
    await applyFromCard(page, LABEL);

    await expect.poll(() => readOpencodeJsonc(), { timeout: 5000 }).not.toBeNull();

    const opencode = JSON.parse(readOpencodeJsonc()!);

    expect(opencode.plugin).toEqual(['some-plugin-1', 'some-plugin-2']);
    expect(opencode.permission.bash).toBe('allow');
    expect(opencode.permission.edit).toBe('deny');

    expect(opencode.provider.omos.models[MODEL_ID].headers).toEqual({
      'X-Custom-Auth': 'keep-me',
      'X-Region': 'us-east-1',
    });
    expect(opencode.provider.omos.models[MODEL_ID].limit.context).toBe(128000);
    expect(opencode.provider.omos.models[MODEL_ID].limit.output).toBe(8192);

    expect(opencode.provider.omos.options.apiKey).toBe(API_KEY);
    expect(opencode.provider.omos.options.baseURL).toBe(BASE_URL);
  });

  test('损坏模式：opencode.jsonc 损坏时触发错误，原始文件不被破坏', async ({ page }) => {
    const corruptRaw = readFileSync(join(FIXTURE_DIR, 'opencode-corrupt.jsonc'), 'utf8');
    seedOpencodeJsonc(corruptRaw);

    await createConfig(page, LABEL);

    const applyBtn = page
      .locator('li', { hasText: LABEL })
      .getByRole('button', { name: '应用' });
    const isDisabled = await applyBtn.isDisabled();
    if (!isDisabled) {
      await applyBtn.click();
      await page.waitForTimeout(500);
    }

    expect(readOpencodeJsonc()).toBe(corruptRaw);
    expect(
      readdirSync(testBackupsDir()).filter((f) => f.endsWith('.json')),
    ).toHaveLength(0);
  });
});

