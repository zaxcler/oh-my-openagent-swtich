/**
 * T17 E2E —— 关键场景 1：新建配置。
 *
 * 流程：启动 app → 点 "+" → 填 label/apiKey/baseURL/model → 保存 → 列表出现新卡片。
 *
 * 数据隔离：
 * - beforeEach 用 `process.env.TAURI_TEST_DATA_DIR` 锁住一个 tmpdir（值由
 *   playwright.config.ts 注入），Tauri 应用通过 OMO_TEST_* 环境变量落同样的目录。
 * - afterEach 清理三个子目录 + active.json，不留尾巴。
 */
import { expect, test } from '@playwright/test';
import { cleanTestData, testDataDir } from './helpers/test-env';
import { maybeSkipOnUnsupportedPlatform } from './helpers/skip';

maybeSkipOnUnsupportedPlatform();

const LABEL = 'e2e-create-config';
const API_KEY = 'sk-e2e-key';
const BASE_URL = 'https://e2e.example.com/v1';
const MODEL_ID = 'e2e-model';
const MODEL_NAME = 'E2E Model';

test.describe('create-config', () => {
  test.beforeEach(() => {
    process.env.TAURI_TEST_DATA_DIR = testDataDir();
    cleanTestData();
  });

  test.afterEach(() => {
    cleanTestData();
  });

  test('初始空状态正确显示「从 opencode 导入」入口', async ({ page }) => {
    await page.goto('/');

    await expect(page.getByRole('heading', { name: '还没有配置' })).toBeVisible();
    await expect(
      page.getByRole('button', { name: '从 opencode 导入' }),
    ).toBeVisible();
  });

  test('新建一份配置后列表立即出现新卡片', async ({ page }) => {
    await page.goto('/');

    await expect(page.getByRole('heading', { name: '还没有配置' })).toBeVisible();

    await page.getByRole('link', { name: '新建配置' }).click();
    await expect(page).toHaveURL(/\/edit(\/new)?$/);
    await expect(page.getByRole('heading', { name: '新建配置' })).toBeVisible();

    await page.getByLabel('配置名称 *').fill(LABEL);
    await page.getByLabel('API Key *').fill(API_KEY);
    await page.getByLabel('Base URL *').fill(BASE_URL);
    await page.getByLabel('model id').fill(MODEL_ID);
    await page.getByLabel('model name').fill(MODEL_NAME);

    await page.getByRole('button', { name: '保存' }).click();
    await expect(page).toHaveURL(/\/$/);

    const card = page.locator('li', { hasText: LABEL });
    await expect(card).toBeVisible();
    await expect(card.getByRole('heading', { name: LABEL })).toBeVisible();
    await expect(card.locator('.badge').first()).toBeVisible();

    await expect(page.getByRole('heading', { name: '还没有配置' })).toHaveCount(0);
  });
});

