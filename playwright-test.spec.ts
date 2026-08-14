import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';

test('Test PlanScreen', async ({ page }) => {
    await page.goto('http://localhost:1420');
    await page.waitForTimeout(2000);
    const html = await page.content();
    console.log("DOM DUMP:", html);
});
