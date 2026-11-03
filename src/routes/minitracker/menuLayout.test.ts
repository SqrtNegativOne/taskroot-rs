import { describe, expect, it } from 'vitest';
import { DEFAULT_BAR_HEIGHT, computeMenuLayout } from './menuLayout';

const WORK_AREA = { top: 0, bottom: 900 };

describe('computeMenuLayout', () => {
    it('opens downward when there is room below the bar', () => {
        expect(computeMenuLayout(100, 2, WORK_AREA)).toEqual({
            placement: 'down',
            menuHeight: 120,
            windowHeight: 120 + DEFAULT_BAR_HEIGHT,
            windowY: 100,
        });
    });

    it('opens upward and keeps the bar in place when the bar is near the bottom', () => {
        expect(computeMenuLayout(800, 8, WORK_AREA)).toEqual({
            placement: 'up',
            menuHeight: 324,
            windowHeight: 324 + DEFAULT_BAR_HEIGHT,
            windowY: 476,
        });
    });

    it('clamps the menu to the space below when neither side fits fully', () => {
        const layout = computeMenuLayout(840, 20, { top: 780, bottom: 900 });

        expect(layout.placement).toBe('up');
        expect(layout.menuHeight).toBe(120);
        expect(layout.windowY).toBe(720);
    });
});
