import { describe, expect, it } from 'vitest';
import { ymd } from '../../../lib/time';
import { DateGridView } from './constants';
import { buildDateGrid, shiftDateGridAnchor } from './dates';

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
