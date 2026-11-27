import { describe, expect, it } from 'vitest';
import {
    formatHhMm,
    isoWeeksInYear,
    parseLocalYmd,
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

describe('startOfWeek', () => {
    it('returns Monday midnight of the containing week', () => {
        const start = startOfWeek(new Date(2026, 9, 28, 15, 30));

        expect(ymd(start)).toBe('2026-10-26');
        expect(start.getHours()).toBe(0);
        expect(start.getMinutes()).toBe(0);
    });

    it('keeps a Monday at its own midnight and crosses year boundaries', () => {
        expect(ymd(startOfWeek(new Date(2026, 9, 26, 23, 59)))).toBe('2026-10-26');
        expect(ymd(startOfWeek(new Date(2026, 0, 1)))).toBe('2025-12-29');
    });
});

describe('isoWeeksInYear', () => {
    it('distinguishes 52- and 53-week ISO years', () => {
        expect(isoWeeksInYear(2025)).toBe(52);
        expect(isoWeeksInYear(2026)).toBe(53);
        expect(isoWeeksInYear(2020)).toBe(53);
    });
});
