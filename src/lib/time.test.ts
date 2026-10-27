import { describe, expect, it } from 'vitest';
import { getWeekNumber, isoWeeksInYear, isoWeekYear, startOfMonth, startOfWeek, ymd } from './time';

describe('startOfWeek', () => {
    it('returns Monday midnight of the containing week', () => {
        const start = startOfWeek(new Date(2026, 9, 28, 15, 30));

        expect(ymd(start)).toBe('2026-10-26');
        expect(start.getHours()).toBe(0);
        expect(start.getMinutes()).toBe(0);
    });

    it('keeps a Monday at its own midnight', () => {
        expect(ymd(startOfWeek(new Date(2026, 9, 26, 23, 59)))).toBe('2026-10-26');
    });

    it('crosses into the previous year when the week starts there', () => {
        expect(ymd(startOfWeek(new Date(2026, 0, 1)))).toBe('2025-12-29');
    });
});

describe('startOfMonth', () => {
    it('returns the first day of the month at midnight', () => {
        const start = startOfMonth(new Date(2026, 9, 28, 15, 30));

        expect(ymd(start)).toBe('2026-10-01');
        expect(start.getHours()).toBe(0);
    });
});

describe('getWeekNumber', () => {
    it('reports the ISO week for a mid-year date', () => {
        expect(getWeekNumber(new Date(2026, 9, 26))).toBe(44);
    });

    it('treats a late-December Monday as week 1 of the next year', () => {
        expect(getWeekNumber(new Date(2025, 11, 29))).toBe(1);
    });

    it('reports week 53 for a year that has one', () => {
        expect(getWeekNumber(new Date(2026, 11, 31))).toBe(53);
    });
});

describe('isoWeeksInYear', () => {
    it('distinguishes 52- and 53-week ISO years', () => {
        expect(isoWeeksInYear(2025)).toBe(52);
        expect(isoWeeksInYear(2026)).toBe(53);
        expect(isoWeeksInYear(2020)).toBe(53);
    });
});

describe('isoWeekYear', () => {
    it('uses the week-year around New Year boundaries', () => {
        expect(isoWeekYear(new Date(2025, 11, 29))).toBe(2026);
        expect(isoWeekYear(new Date(2027, 0, 1))).toBe(2026);
        expect(isoWeekYear(new Date(2026, 9, 26))).toBe(2026);
    });
});
