import { test, expect } from '@playwright/test';
import type { AppSettings } from '../../src/lib/bindings/AppSettings.generated';

const settings: AppSettings = {
  default_calendar_view: 'month',
  day_timeline_start_view: 1,
  default_task_duration: 30,
  clock_style: 'flowtime',
  allow_stopwatch_without_task: true,
  flowtime_break_divisor: 5,
  pause_minutes: 5,
  mini_tracker_enabled: false,
  keybinding_launcher: '',
  keybinding_open_settings: '',
  keybinding_restore_app: '',
  tracker_show_border: false,
  tracker_opacity: 1,
};

test('spa mounts the main window and routes an unconfigured account to connect', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.addInitScript((appSettings) => {
    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      value: {
        metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
        transformCallback: () => 1,
        unregisterCallback: () => undefined,
        invoke: (command: string) => {
          if (command === 'get_settings') return Promise.resolve(appSettings);
          if (command === 'is_account_configured') return Promise.resolve(false);
          if (command === 'plugin:event|listen') return Promise.resolve(1);
          return Promise.resolve(undefined);
        },
      },
    });
    Object.defineProperty(window, '__TAURI_EVENT_PLUGIN_INTERNALS__', {
      value: { unregisterListener: () => undefined },
    });
  }, settings);

  await page.goto('/');

  await expect(page.locator('.app')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Connect CalDAV' })).toBeVisible();
  await expect(page).toHaveURL(/\/connect$/);
  expect(errors).toEqual([]);
});
