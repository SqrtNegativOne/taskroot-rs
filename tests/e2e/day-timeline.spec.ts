import { readFile } from 'node:fs/promises';
import { test, expect } from '@playwright/test';

const stylesheet = await readFile('src/screens/plan/day-timeline/day-timeline.css', 'utf8');

for (const columnWidth of [200, 480]) {
  for (const labelOffset of [0, 56]) {
    for (const lanes of [1, 3]) {
      test(`timeline leaves creation space after each lane (${columnWidth}px, gutter ${labelOffset}px, ${lanes} lanes)`, async ({ page }) => {
        await page.setContent(`
          <style>
            * { box-sizing: border-box; }
            ${stylesheet}
            .day-grid { width: ${columnWidth}px; height: 200px; --day-label-offset: ${labelOffset}px; }
            .day-event { top: 0; height: 100px; }
          </style>
          <div class="day-grid">
            ${Array.from({ length: lanes }, (_, lane) => `
              <div class="day-event" style="--event-lane: ${lane}; --event-lanes: ${lanes};">Event</div>
            `).join('')}
          </div>
        `);

        const grid = await page.locator('.day-grid').boundingBox();
        expect(grid).not.toBeNull();
        if (!grid) return;

        const laneWidth = (grid.width - labelOffset) / lanes;
        for (let lane = 0; lane < lanes; lane++) {
          const event = await page.locator('.day-event').nth(lane).boundingBox();
          expect(event).not.toBeNull();
          if (!event) return;

          const laneStart = grid.x + labelOffset + lane * laneWidth;
          const laneEnd = laneStart + laneWidth;
          expect(event.x).toBeCloseTo(laneStart, 0);
          expect(event.width).toBeGreaterThan(0);
          expect(laneEnd - (event.x + event.width)).toBeGreaterThanOrEqual(20);

          const hit = await page.evaluate(({ x, y }) =>
            document.elementFromPoint(x, y)?.classList.contains('day-grid'),
          { x: laneEnd - 10, y: grid.y + 50 });
          expect(hit).toBe(true);
        }
      });
    }
  }
}

test('timeline events only retain their thick left accent border', async ({ page }) => {
  const variants = ['', 'ev-plan', 'ev-meeting', 'is-done', 'is-ghost', 'is-floating', 'is-readonly', 'is-compact'];
  await page.setContent(`
    <style>${stylesheet}</style>
    ${variants.map((variant) => `
      <div class="day-event ${variant}" style="--ev-color: rgb(100, 150, 200); --border: black; --border-strong: black; --accent-line: red; --accent: red;">Event</div>
    `).join('')}
  `);

  for (const event of await page.locator('.day-event').all()) {
    const border = await event.evaluate((element) => {
      const style = getComputedStyle(element);
      return [style.borderTopWidth, style.borderRightWidth, style.borderBottomWidth, style.borderLeftWidth];
    });
    expect(border).toEqual(['0px', '0px', '0px', '3px']);
  }
});
