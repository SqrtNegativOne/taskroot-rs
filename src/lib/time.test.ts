import { describe, expect, it } from 'vitest';
import {
    addDays,
    dayDiff,
    formatHhMm,
    getWeekNumber,
    isoWeeksInYear,
    isoWeekYear,
    parseLocalYmd,
    sameDay,
    startOfMonth,
    startOfWeek,
    ymd,
} from './time';

describe('parseLocalYmd', () => {
    it('parses a date string at local midnight', () => {
        const d = parseLocalYmd('2026-10-26');

        expect(ymd(d)).toBe('2026-10-26');
        expect(d.getHours()).toBe(0);
        expect(d.getMinutes()).toBe(0);
        expect(d.getSeconds()).toBe(0);
    });
});

describe('formatHhMm', () => {
    it('zero-pads hour and minute in 24-hour form', () => {
        expect(formatHhMm(0)).toBe('00:00');
        expect(formatHhMm(9 * 60 + 5)).toBe('09:05');
        expect(formatHhMm(14 * 60 + 30)).toBe('14:30');
        expect(formatHhMm(23 * 60 + 59)).toBe('23:59');
    });

    it('rounds a fractional minute before formatting', () => {
        expect(formatHhMm(149.6)).toBe('02:30');
    });
});

describe('addDays', () => {
    it('advances the calendar date while keeping the local time of day', () => {
        const next = addDays(new Date(2026, 11, 31, 9, 15), 1);

        expect(ymd(next)).toBe('2027-01-01');
        expect(next.getHours()).toBe(9);
        expect(next.getMinutes()).toBe(15);
    });
});

describe('dayDiff', () => {
    it('counts calendar days rather than elapsed milliseconds', () => {
        expect(dayDiff(new Date(2026, 10, 2, 0, 30), new Date(2026, 10, 1, 23, 30))).toBe(1);
        expect(dayDiff(new Date(2026, 10, 1, 23, 30), new Date(2026, 10, 2, 0, 30))).toBe(-1);
    });
});

describe('sameDay', () => {
    it('compares the local calendar date', () => {
        expect(sameDay(new Date(2026, 9, 26, 0, 1), new Date(2026, 9, 26, 23, 59))).toBe(true);
        expect(sameDay(new Date(2026, 9, 26, 23, 59), new Date(2026, 9, 27, 0, 1))).toBe(false);
    });
});

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
