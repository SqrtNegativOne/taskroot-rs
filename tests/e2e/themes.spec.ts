import { readFile } from 'node:fs/promises';
import { test, expect } from '@playwright/test';

const css = (await Promise.all([
    'src/lib/themes/slate.css',
    'src/lib/themes/ember.css',
    'src/lib/themes/tokens.css',
    'src/components/tasklist/tasklist.css',
    'src/screens/plan/date-grid/date-grid.css',
    'src/screens/plan/day-timeline/day-timeline.css',
    'src/screens/plan/day-timeline/timeline-events.css',
    'src/routes/minitracker/minitracker.css',
    'src/routes/minitracker/task-menu.css',
    'src/screens/sidebar/sidebar.css',
].map((path) => readFile(path, 'utf8')))).join('\n');

const fixture = `
    <style>${css}</style>
    <div id="surface" style="background: var(--bg-surface); color: var(--fg)">Text</div>
    <div id="muted" style="color: var(--fg-muted)">Secondary text</div>
    <button id="accent" style="background: var(--accent); color: var(--on-accent)">Action</button>
    <div class="day-cell is-today"><span class="day-cell-num">Today</span><span class="day-cell-today-flag">Today</span></div>
    <div class="day-head-cell is-today">Today</div>
    <div class="day-gutter-hours"><span class="day-now-label">Now</span></div>
    <div class="day-now-line"></div>
    <span class="task-row-start-icon is-today">Starts today</span>
    <span class="pri-2">Priority</span>
    <div class="minitracker-container"><button class="minitracker-task has-task">Tracking</button></div>
    <div class="task-menu"><span class="task-badge">Current</span></div>
    <div class="sidebar-widget"><button class="tab">Sidebar</button></div>
    <div class="day-event" style="--ev-color: #a03260; --ev-bg: color-mix(in srgb, var(--bg-surface) 82%, #a03260); --ev-text: var(--fg); --ev-secondary: var(--fg-muted)">Server colour</div>
`;

function luminance(rgb: number[]): number {
    const channels = rgb.map((value) => {
        const channel = value / 255;
        return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
    });
    return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
}

function contrast(first: number[], second: number[]): number {
    const a = luminance(first);
    const b = luminance(second);
    return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

for (const scheme of ['slate', 'ember']) {
    test(`${scheme} keeps primary, secondary and action text readable`, async ({ page }) => {
        await page.setContent(fixture);
        await page.evaluate((theme) => { document.documentElement.dataset.theme = theme; }, scheme);
        const colours = await page.evaluate(() => {
            function rgb(selector: string, property: 'color' | 'backgroundColor'): number[] {
                const element = document.querySelector(selector);
                const context = document.createElement('canvas').getContext('2d');
                if (!element || !context) throw new Error('Missing colour probe');
                context.fillStyle = getComputedStyle(element)[property];
                context.fillRect(0, 0, 1, 1);
                return Array.from(context.getImageData(0, 0, 1, 1).data).slice(0, 3);
            }
            return {
                surface: rgb('#surface', 'backgroundColor'),
                text: rgb('#surface', 'color'),
                muted: rgb('#muted', 'color'),
                accent: rgb('#accent', 'backgroundColor'),
                onAccent: rgb('#accent', 'color'),
            };
        });
        expect(contrast(colours.text, colours.surface)).toBeGreaterThanOrEqual(4.5);
        expect(contrast(colours.muted, colours.surface)).toBeGreaterThanOrEqual(4.5);
        expect(contrast(colours.onAccent, colours.accent)).toBeGreaterThanOrEqual(4.5);
        if (scheme === 'slate') {
            expect(luminance(colours.surface)).toBeGreaterThan(0.02);
            expect(luminance(colours.surface)).toBeLessThan(0.08);
            expect(colours.accent[2]).toBeGreaterThan(colours.accent[0]);
        }
    });
}

test('today and current-time markers stay red when the theme changes', async ({ page }) => {
    await page.setContent(fixture);
    const markers = [
        ['.day-cell-num', 'backgroundColor'],
        ['.day-cell-today-flag', 'color'],
        ['.day-head-cell.is-today', 'color'],
        ['.day-now-label', 'color'],
        ['.day-now-line', 'borderTopColor'],
        ['.task-row-start-icon.is-today', 'color'],
    ];
    let previous: number[][] | undefined;
    for (const scheme of ['slate', 'ember']) {
        await page.evaluate((theme) => { document.documentElement.dataset.theme = theme; }, scheme);
        const colours = await page.evaluate((probes) => probes.map(([selector, property]) => {
            const element = document.querySelector(selector);
            const context = document.createElement('canvas').getContext('2d');
            if (!element || !context) throw new Error('Missing today marker');
            context.fillStyle = getComputedStyle(element).getPropertyValue(
                property.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`),
            );
            context.fillRect(0, 0, 1, 1);
            return Array.from(context.getImageData(0, 0, 1, 1).data).slice(0, 3);
        }), markers);
        for (const [red, green, blue] of colours) {
            expect(red).toBeGreaterThan(green * 1.5);
            expect(red).toBeGreaterThan(blue * 1.5);
        }
        if (previous) expect(colours).toEqual(previous);
        previous = colours;
    }
});

test('switching palettes recolours app surfaces and event fills but preserves the server colour bar', async ({ page }) => {
    await page.setContent(fixture);
    const probes = ['#surface', '#accent', '.pri-2', '.minitracker-task', '.task-badge', '.tab'];
    const eventFill = () => page.locator('.day-event').evaluate((element) => getComputedStyle(element).backgroundColor);
    const before = await page.locator(probes.join(',')).evaluateAll((elements) => elements.map((element) => {
        const style = getComputedStyle(element);
        return `${style.color} ${style.backgroundColor}`;
    }));
    const fillBefore = await eventFill();

    await page.evaluate(() => { document.documentElement.dataset.theme = 'ember'; });
    const after = await page.locator(probes.join(',')).evaluateAll((elements) => elements.map((element) => {
        const style = getComputedStyle(element);
        return `${style.color} ${style.backgroundColor}`;
    }));
    expect(after).toHaveLength(before.length);
    for (let index = 0; index < before.length; index += 1) expect(after[index]).not.toBe(before[index]);
    expect(await eventFill()).not.toBe(fillBefore);
    await expect(page.locator('.day-event')).toHaveCSS('border-left-color', 'rgb(160, 50, 96)');
});
