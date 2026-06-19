import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: 'html',
  use: {
    baseURL: 'tauri://localhost',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'tauri',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});
