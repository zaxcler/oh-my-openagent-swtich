/**
 * 跨 spec 复用的小工具。
 *
 * 目前只放 `maybeSkipOnUnsupportedPlatform`——把"在 macOS 上整体跳过"集中起来，
 * 这样每个 spec 顶部都有一行显式 skip，跟 playwright.config 的 testIgnore 兜底。
 */
import { test } from '@playwright/test';

export function maybeSkipOnUnsupportedPlatform(): void {
  test.skip(
    process.platform === 'darwin',
    'tauri-driver WKWebView not supported on macOS without tauri-plugin-automation',
  );
}
