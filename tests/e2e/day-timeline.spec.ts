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

test('timeline events round their corners without gaining any other border', async ({ page }) => {
  const variants = ['', 'ev-plan', 'ev-meeting', 'is-done', 'is-ghost', 'is-floating', 'is-readonly', 'is-compact'];
  await page.setContent(`
    <style>${stylesheet}</style>
    ${variants.map((variant) => `
      <div class="day-event ${variant}" style="--ev-color: rgb(100, 150, 200); --border: black; --border-strong: black; --accent-line: red; --accent: red;">Event</div>
    `).join('')}
  `);

  for (const event of await page.locator('.day-event').all()) {
    const style = await event.evaluate((element) => {
      const computed = getComputedStyle(element);
      return {
        radius: computed.borderRadius,
        borders: [computed.borderTopWidth, computed.borderRightWidth, computed.borderBottomWidth, computed.borderLeftWidth],
      };
    });
    expect(Number.parseFloat(style.radius)).toBeGreaterThan(0);
    expect(style.borders).toEqual(['0px', '0px', '0px', '3px']);
  }
});

function dayTimelineFixture(css: string): string {
  return `
    <style>
      * { box-sizing: border-box; }
      ${stylesheet}
      ${css}
    </style>
    <div class="day-scroll">
      <div class="day-cols">
        <div class="day-head">
          <div class="day-head-cell" data-day="d1">Mon, Nov 2</div>
          <div class="day-head-cell" data-day="d2">Tue, Nov 3</div>
          <div class="day-head-cell" data-day="d3">Wed, Nov 4</div>
        </div>
        <div class="day-row">
          <div class="day-grid" data-drop-date="d1" style="flex: 1; min-width: 200px; height: 1300px;"></div>
          <div class="day-grid" data-drop-date="d2" style="flex: 1; min-width: 200px; height: 1300px;"></div>
          <div class="day-grid" data-drop-date="d3" style="flex: 1; min-width: 200px; height: 1300px;"></div>
        </div>
      </div>
    </div>
  `;
}

test('day headers stay pinned directly above their columns while the timeline scrolls', async ({ page }) => {
  await page.setContent(dayTimelineFixture('.day-scroll { width: 500px; height: 300px; }'));

  const position = await page.locator('.day-head').evaluate((element) => getComputedStyle(element).position);
  expect(position).toBe('sticky');

  const cells = await page.locator('.day-head-cell').all();
  const grids = await page.locator('.day-grid').all();
  expect(cells).toHaveLength(3);
  for (let index = 0; index < cells.length; index += 1) {
    const cellBox = await cells[index].boundingBox();
    const gridBox = await grids[index].boundingBox();
    if (!cellBox || !gridBox) throw new Error('missing bounding box');
    expect(cellBox.x).toBeCloseTo(gridBox.x, 0);
    expect(cellBox.width).toBeCloseTo(gridBox.width, 0);
  }

  const scrollerBox = await page.locator('.day-scroll').boundingBox();
  if (!scrollerBox) throw new Error('missing bounding box');
  await page.locator('.day-scroll').evaluate((element) => {
    element.scrollTop = 400;
  });

  const headerBox = await page.locator('.day-head').boundingBox();
  if (!headerBox) throw new Error('missing bounding box');
  expect(headerBox.y).toBeCloseTo(scrollerBox.y, 0);
  const gridBox = await grids[0].boundingBox();
  if (!gridBox) throw new Error('missing bounding box');
  expect(gridBox.y).toBeLessThan(scrollerBox.y - 100);
});

test('the timeline snaps sideways onto a day boundary after a horizontal scroll', async ({ page }) => {
  await page.goto('/');
  await page.evaluate((css) => {
    document.body.innerHTML = css;
  }, dayTimelineFixture('.day-scroll { width: 400px; height: 300px; }'));

  await page.addScriptTag({
    type: 'module',
    content: `import { attachDaySnap } from '/src/screens/plan/day-timeline/scrollSnap.ts';
      attachDaySnap(document.querySelector('.day-scroll'));`,
  });

  const scroller = page.locator('.day-scroll');
  const box = await scroller.boundingBox();
  if (!box) throw new Error('missing bounding box');
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.wheel(130, 0);

  await expect.poll(() => scroller.evaluate((element) => element.scrollLeft)).toBe(200);
});
