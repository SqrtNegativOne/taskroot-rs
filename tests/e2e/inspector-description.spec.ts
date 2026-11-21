import { readFile } from 'node:fs/promises';
import { test, expect } from '@playwright/test';

const stylesheet = await readFile('src/components/inspector-pane/inspector.css', 'utf8');

test('a long unbroken description wraps inside the field instead of overflowing', async ({ page }) => {
  const longWord = 'Supercalifragilisticexpialidocious'.repeat(6);

  await page.setContent(`
    <style>
      * { box-sizing: border-box; }
      body { margin: 0; }
      ${stylesheet}
    </style>
    <div class="inspector-pane is-open" style="position: relative; width: 380px; height: 400px; transform: none;">
      <div class="inspector-body">
        <div class="inspector-field is-multiline" data-field="Description">
          <span class="inspector-field-icon"></span>
          <div class="inspector-field-value">
            <div class="description-input">
              <button type="button" class="inspector-description-display" aria-label="Description">${longWord}</button>
            </div>
          </div>
        </div>
      </div>
    </div>
  `);

  const pane = page.locator('.inspector-pane');
  const display = page.locator('.inspector-description-display');

  await expect(display).toBeVisible();

  const paneBox = await pane.boundingBox();
  const fieldBox = await page.locator('[data-field="Description"]').boundingBox();
  const displayBox = await display.boundingBox();
  expect(paneBox).not.toBeNull();
  expect(fieldBox).not.toBeNull();
  expect(displayBox).not.toBeNull();
  if (!paneBox || !fieldBox || !displayBox) return;

  const wrapStyle = await display.evaluate((element) => {
    const style = getComputedStyle(element);
    return { whiteSpace: style.whiteSpace, overflowWrap: style.overflowWrap };
  });
  expect(wrapStyle).toEqual({ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' });

  expect(displayBox.width).toBeLessThanOrEqual(fieldBox.width + 1);
  expect(displayBox.x + displayBox.width).toBeLessThanOrEqual(paneBox.x + paneBox.width + 1);

  // Several visual lines: the one unbroken word must have been split across rows.
  const lines = await display.evaluate((element) => {
    const range = document.createRange();
    range.selectNodeContents(element);
    return range.getClientRects().length;
  });
  expect(lines).toBeGreaterThan(2);
});
