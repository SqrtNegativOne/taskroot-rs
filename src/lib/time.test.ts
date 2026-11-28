import { describe, expect, it } from 'vitest';
import {
    formatHhMm,
    formatLocalDateTime,
    isoWeeksInYear,
    minutesSinceMidnight,
    parseLocalYmd,
    startOfWeek,
    toWallClockParts,
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

describe('formatLocalDateTime and toWallClockParts', () => {
    it('formats local dates as canonical wall-clock YYYY-MM-DDTHH:MM:SS without a Z suffix', () => {
        expect(formatLocalDateTime(new Date(2026, 8, 7, 9, 5, 3))).toBe('2026-09-07T09:05:03');
        expect(formatLocalDateTime(new Date(2026, 10, 5, 0, 0, 0))).toBe('2026-11-05T00:00:00');
    });

    it('keeps date-only and naive wall-clock timestamps literal regardless of timezone', () => {
        expect(toWallClockParts('2026-11-05')).toEqual({ date: '2026-11-05', time: '' });
        expect(toWallClockParts('2026-11-05T09:30:00', 'America/New_York')).toEqual({
            date: '2026-11-05',
            time: '09:30',
        });
    });

    it('converts UTC and offset instants into the named IANA timezone across midnight boundaries', () => {
        expect(toWallClockParts('2026-11-05T23:30:00Z', 'Asia/Kolkata')).toEqual({
            date: '2026-11-06',
            time: '05:00',
        });
        expect(toWallClockParts('2026-11-05T01:30:00Z', 'America/Chicago')).toEqual({
            date: '2026-11-04',
            time: '19:30',
        });
        expect(toWallClockParts('2026-11-05T14:30:00+02:00', 'America/New_York')).toEqual({
            date: '2026-11-05',
            time: '07:30',
        });
    });

    it('converts UTC instants to host local wall clock when timezone is omitted or UTC', () => {
        const instant = new Date('2026-11-05T14:30:00Z');
        const expected = {
            date: ymd(instant),
            time: formatHhMm(minutesSinceMidnight(instant)),
        };

        expect(toWallClockParts('2026-11-05T14:30:00Z')).toEqual(expected);
        expect(toWallClockParts('2026-11-05T14:30:00Z', 'UTC')).toEqual(expected);
    });
});
