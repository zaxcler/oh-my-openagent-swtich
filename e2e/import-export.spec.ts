/**
 * T17 E2E —— 关键场景 4：导入 / 导出。
 *
 * 流程：创建配置 → 通过 Tauri IPC 把它导出到 /tmp → 删掉 → 通过 IPC
 * 把同一份 JSON 重新导入 → 验证 label / provider 内容一致。
 *
 * 关于 dialog：
 *   前端走的是 @tauri-apps/plugin-dialog 的 save / open（OS 原生对话框），
 *   在 WebDriver 下没有可移植的处理方式。本 spec 改为绕过 dialog，直接通过
 *   Tauri IPC 调用 `export_config` / `import_config_file`，专注验证数据回环。
 *   UI 层（按钮存在 / enabled）则通过可见性断言兜底。
 */
import { existsSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, test } from '@playwright/test';
import { cleanTestData, testDataDir } from './helpers/test-env';
import { maybeSkipOnUnsupportedPlatform } from './helpers/skip';

maybeSkipOnUnsupportedPlatform();

const LABEL = 'e2e-import-export';
const API_KEY = 'sk-imex-key';
const BASE_URL = 'https://imex.example.com/v1';
const MODEL_ID = 'imex-model';
const MODEL_NAME = 'Import Export Model';

test.describe('import-export', () => {
  test.beforeEach(() => {
    process.env.TAURI_TEST_DATA_DIR = testDataDir();
    cleanTestData();
  });

  test.afterEach(() => {
    cleanTestData();
  });

  test('UI 上导出 / 导入按钮可见', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: '新建配置' }).click();
    await page.getByLabel('配置名称 *').fill(LABEL);
    await page.getByLabel('API Key *').fill(API_KEY);
    await page.getByLabel('Base URL *').fill(BASE_URL);
    await page.getByLabel('model id').fill(MODEL_ID);
    await page.getByLabel('model name').fill(MODEL_NAME);
    await page.getByRole('button', { name: '保存' }).click();

    const card = page.locator('li', { hasText: LABEL });
    await expect(card.getByRole('button', { name: '导出' })).toBeVisible();
    await expect(card.getByRole('button', { name: '导入' })).toBeVisible();
  });

  test('通过 IPC 验证：export → delete → import → 内容一致', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: '新建配置' }).click();
    await page.getByLabel('配置名称 *').fill(LABEL);
    await page.getByLabel('API Key *').fill(API_KEY);
    await page.getByLabel('Base URL *').fill(BASE_URL);
    await page.getByLabel('model id').fill(MODEL_ID);
    await page.getByLabel('model name').fill(MODEL_NAME);
    await page.getByRole('button', { name: '保存' }).click();

    const card = page.locator('li', { hasText: LABEL });
    const id = await page.evaluate(async (label) => {
      const mod = await import('/src/lib/tauri.ts');
      const list = await mod.tauriInvoke<{ id: string; label: string }[]>(
        'list_configs',
      );
      const found = list.find((c) => c.label === label);
      if (!found) throw new Error(`config ${label} not found`);
      return found.id;
    }, LABEL);
    expect(id).toBeTruthy();

    // 导出到 /tmp 文件
    const exportPath = join(tmpdir(), `omo-e2e-export-${Date.now()}.json`);
    if (existsSync(exportPath)) rmSync(exportPath);
    await page.evaluate(
      async ({ id, target }) => {
        const mod = await import('/src/lib/tauri.ts');
        await mod.tauriInvoke('export_config', { id, target });
      },
      { id, target: exportPath },
    );
    expect(existsSync(exportPath)).toBe(true);
    const exported = JSON.parse(readFileSync(exportPath, 'utf8'));
    expect(exported.label).toBe(LABEL);
    expect(exported.payload.provider.options.api_key).toBe(API_KEY);
    expect(exported.payload.provider.options.base_url).toBe(BASE_URL);

    // 走 UI 删除
    await card.getByRole('button', { name: '删除' }).click();
    await page
      .locator('dialog.modal[open]')
      .getByRole('button', { name: '删除' })
      .click();
    await expect(card).toHaveCount(0);

    // 重新导入
    await page.evaluate(async (path) => {
      const mod = await import('/src/lib/tauri.ts');
      const created = await mod.tauriInvoke<{ id: string; label: string }>(
        'import_config_file',
        { path },
      );
      if (!created) throw new Error('import_config_file returned null');
    }, exportPath);

    // 列表里应出现「imported-...」或「e2e-import-export」label（import_config_file 用原 label 写入）
    await expect(page.locator('li', { hasText: LABEL })).toBeVisible();

    // 清理临时导出文件
    rmSync(exportPath, { force: true });
  });
});
