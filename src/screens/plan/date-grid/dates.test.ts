import { describe, expect, it } from 'vitest';
import { ymd } from '../../../lib/time';
import { DateGridView } from './constants';
import {
    buildDateGrid,
    buildInfiniteWeeks,
    dominantMonthInViewport,
    shiftDateGridAnchor,
} from './dates';

describe('date-grid dates', () => {
    it.each([
        { view: DateGridView.OneWeek, length: 7, end: '2026-09-13' },
        { view: DateGridView.ThreeWeeks, length: 21, end: '2026-09-27' },
    ])('queries exactly the displayed dates in $view', ({ view, length, end }) => {
        const { cells, range } = buildDateGrid(new Date(2026, 8, 9, 15), view);
        expect(cells).toHaveLength(length);
        expect(cells.every(cell => !cell.outOfMonth)).toBe(true);
        expect(ymd(cells[0].date)).toBe('2026-09-07');
        expect(ymd(cells[cells.length - 1].date)).toBe(end);
        expect(range).toEqual({ startDate: '2026-09-07', endDate: end });
    });

    it('includes month spillover days in the display and query', () => {
        const { cells, range } = buildDateGrid(new Date(2026, 8, 9), DateGridView.Month);
        expect(cells).toHaveLength(42);
        expect(range).toEqual({ startDate: '2026-08-31', endDate: '2026-10-11' });
        expect(cells[0].outOfMonth).toBe(true);
        expect(cells[1].outOfMonth).toBe(false);
        expect(cells[41].outOfMonth).toBe(true);
    });

    it.each([
        { anchor: new Date(2026, 0, 31), step: 1, expected: '2026-02-28' },
        { anchor: new Date(2028, 0, 31), step: 1, expected: '2028-02-29' },
        { anchor: new Date(2026, 2, 31), step: -1, expected: '2026-02-28' },
    ])('does not skip short months ($expected)', ({ anchor, step, expected }) => {
        expect(ymd(shiftDateGridAnchor(anchor, DateGridView.Month, step))).toBe(expected);
        expect(anchor.getDate()).toBe(31);
    });

    it.each([
        { view: DateGridView.OneWeek, expected: '2026-09-16' },
        { view: DateGridView.ThreeWeeks, expected: '2026-09-30' },
    ])('moves $view by its displayed length', ({ view, expected }) => {
        expect(ymd(shiftDateGridAnchor(new Date(2026, 8, 9), view, 1))).toBe(expected);
    });
});

describe('dominantMonthInViewport', () => {
    // 14 consecutive weeks starting Mon 2026-10-26 (covers Nov, Dec 2026, Jan 2027).
    const weeks = buildInfiniteWeeks(new Date(2026, 9, 26), 14);
    const weekHeight = 100;
    const viewportHeight = 600; // 6 visible week rows

    it('identifies the primary month when framed at its first week', () => {
        expect(
            dominantMonthInViewport(weeks, 0, viewportHeight, weekHeight, { year: 2026, month: 9 }),
        ).toEqual({ year: 2026, month: 10 });
    });

    it('switches to the next month once it occupies more visible rows while scrolling down', () => {
        // At scrollTop = 200 (weeks 2..7), Nov has 22 days vs Dec 20 days -> Nov still wins.
        expect(
            dominantMonthInViewport(weeks, 200, viewportHeight, weekHeight, { year: 2026, month: 10 }),
        ).toEqual({ year: 2026, month: 10 });

        // Scrolling 25px further down flips the visible area in favor of December.
        expect(
            dominantMonthInViewport(weeks, 225, viewportHeight, weekHeight, { year: 2026, month: 10 }),
        ).toEqual({ year: 2026, month: 11 });
    });

    it('tracks year transitions cleanly across December into January', () => {
        // Week 9 starts Mon 2026-12-28; 6 visible rows from week 9 are mostly January 2027.
        expect(
            dominantMonthInViewport(weeks, 900, viewportHeight, weekHeight, { year: 2026, month: 11 }),
        ).toEqual({ year: 2027, month: 0 });
    });

    it('returns the fallback when the viewport has not been measured yet', () => {
        const fallback = { year: 2026, month: 10 };
        expect(dominantMonthInViewport(weeks, 0, 0, 0, fallback)).toEqual(fallback);
    });
});
