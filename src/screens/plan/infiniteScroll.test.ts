import { describe, expect, it } from 'vitest';
import {
    canScrollWithinWindow,
    leadingItemIndex,
    recenterShift,
    settleRecenterShift,
    windowLength,
    type InfiniteWindowConfig,
} from './infiniteScroll';

const config: InfiniteWindowConfig = {
    visibleCount: 3,
    bufferCount: 7,
    recenterMargin: 3,
};

describe('infiniteScroll window math', () => {
    it('sizes the rendered window as visible plus symmetric buffers', () => {
        expect(windowLength(config)).toBe(17);
    });

    it('keeps the window anchored while scroll stays inside the safe band', () => {
        expect(recenterShift(700, 100, config)).toBe(0);
        expect(recenterShift(300, 100, config)).toBe(0);
        expect(recenterShift(1100, 100, config)).toBe(0);
    });

    it('shifts forward when scrolled past the right/bottom recenter margin', () => {
        expect(recenterShift(1120, 100, config)).toBe(4);
        expect(recenterShift(1400, 100, config)).toBe(7);
    });

    it('shifts backward when scrolled past the left/top recenter margin', () => {
        expect(recenterShift(280, 100, config)).toBe(-4);
        expect(recenterShift(0, 100, config)).toBe(-7);
    });

    it('does not shift when item size has not been measured yet', () => {
        expect(recenterShift(0, 0, config)).toBe(0);
    });

    it('clamps leadingItemIndex to the scrollable bounds', () => {
        expect(leadingItemIndex(-50, 100, 14)).toBe(0);
        expect(leadingItemIndex(260, 100, 14)).toBe(3);
        expect(leadingItemIndex(9999, 100, 14)).toBe(14);
        expect(leadingItemIndex(260, 0, 14)).toBe(0);
    });

    it('reports whether a target index can scroll smoothly inside the current buffer', () => {
        expect(canScrollWithinWindow(0, config)).toBe(true);
        expect(canScrollWithinWindow(14, config)).toBe(true);
        expect(canScrollWithinWindow(-1, config)).toBe(false);
        expect(canScrollWithinWindow(15, config)).toBe(false);
    });

    it('only recenters on settle when drifted at least the settle threshold away from center', () => {
        expect(settleRecenterShift(700, 100, config, 2)).toBe(0);
        expect(settleRecenterShift(800, 100, config, 2)).toBe(0);
        expect(settleRecenterShift(900, 100, config, 2)).toBe(2);
        expect(settleRecenterShift(500, 100, config, 2)).toBe(-2);
    });
});
