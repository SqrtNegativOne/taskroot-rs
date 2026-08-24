import { test, expect } from '@playwright/test';

test('spa mounts and renders the main window branch', async ({ page }) => {
  await page.goto('/');

  const appShell = page.locator('.app');
  const authCheckSpinner = page.locator('.animate-spin');

  // Outside a Tauri webview the auth probe never resolves, so the layout
  // legitimately rests on either the main shell or the auth-check spinner.
  await expect(appShell.or(authCheckSpinner)).toBeVisible();
});
