import { describe, expect, it } from 'vitest';
import { DEFAULT_BAR_HEIGHT, computeMenuLayout } from './menuLayout';

const WORK_AREA = { top: 0, bottom: 900 };

describe('computeMenuLayout', () => {
    it('opens downward and keeps the bar anchored when there is room below', () => {
        const layout = computeMenuLayout(100, 2, WORK_AREA);

        expect(layout.placement).toBe('down');
        expect(layout.windowY).toBe(100);
        expect(layout.windowHeight).toBe(layout.menuHeight + DEFAULT_BAR_HEIGHT);
        expect(layout.windowY + layout.windowHeight).toBeLessThanOrEqual(WORK_AREA.bottom);
    });

    it('opens upward and keeps the bar in place when near the bottom', () => {
        const layout = computeMenuLayout(800, 8, WORK_AREA);

        expect(layout.placement).toBe('up');
        expect(layout.windowY + layout.menuHeight).toBe(800);
        expect(layout.windowHeight).toBe(layout.menuHeight + DEFAULT_BAR_HEIGHT);
        expect(layout.windowY).toBeGreaterThanOrEqual(WORK_AREA.top);
    });

    it('flips upward when extra form content no longer fits below', () => {
        const compact = computeMenuLayout(700, 2, WORK_AREA);
        const expanded = computeMenuLayout(700, 2, WORK_AREA, DEFAULT_BAR_HEIGHT, 300);

        expect(compact.placement).toBe('down');
        expect(expanded.placement).toBe('up');
        expect(expanded.menuHeight).toBeGreaterThan(compact.menuHeight);
        expect(expanded.windowY + expanded.menuHeight).toBe(700);
    });

    it('picks the roomier side and keeps a minimum usable height when cramped', () => {
        const layout = computeMenuLayout(840, 20, { top: 780, bottom: 900 });

        expect(layout.placement).toBe('up');
        expect(layout.menuHeight).toBeGreaterThan(0);
        expect(layout.windowY + layout.menuHeight).toBe(840);
        expect(layout.windowHeight).toBe(layout.menuHeight + DEFAULT_BAR_HEIGHT);
    });
});
