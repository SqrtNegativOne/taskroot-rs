import { describe, expect, it } from 'vitest';
import { ymd } from '../../lib/time';
import {
    formatWrapDateLabel,
    isPostMidnightWrap,
    isWrapWindowOpen,
    tomorrowOf,
    wrapTargetDate,
} from './wrapWindow';

describe('isWrapWindowOpen', () => {
    it.each([
        ['21:59:59 is closed', new Date(2026, 10, 26, 21, 59, 59), false],
        ['22:00:00 opens the window', new Date(2026, 10, 26, 22, 0, 0), true],
        ['23:59:59 stays open', new Date(2026, 10, 26, 23, 59, 59), true],
        ['00:00:00 stays open across midnight', new Date(2026, 10, 27, 0, 0, 0), true],
        ['00:01:00 stays open', new Date(2026, 10, 27, 0, 1, 0), true],
        ['02:59:59 is the last open minute', new Date(2026, 10, 27, 2, 59, 59), true],
        ['03:00:00 closes the window', new Date(2026, 10, 27, 3, 0, 0), false],
        ['12:00:00 noon is closed', new Date(2026, 10, 27, 12, 0, 0), false],
    ])('%s', (_label, date, expected) => {
        expect(isWrapWindowOpen(date)).toBe(expected);
    });
});

describe('wrapTargetDate & tomorrowOf', () => {
    it('returns today during the pre-midnight evening window (10 PM - 11:59 PM)', () => {
        const evening = new Date(2026, 10, 26, 23, 30, 0);
        const target = wrapTargetDate(evening);

        expect(ymd(target)).toBe('2026-11-26');
        expect(target.getHours()).toBe(0);
        expect(target.getMinutes()).toBe(0);
        expect(isPostMidnightWrap(evening)).toBe(false);
        expect(ymd(tomorrowOf(target))).toBe('2026-11-27');
    });

    it('returns yesterday after midnight before 3 AM (e.g. 12:01 AM and 2:45 AM)', () => {
        const earlyA = new Date(2026, 10, 27, 0, 1, 0);
        const earlyB = new Date(2026, 10, 27, 2, 45, 0);

        expect(ymd(wrapTargetDate(earlyA))).toBe('2026-11-26');
        expect(ymd(wrapTargetDate(earlyB))).toBe('2026-11-26');
        expect(isPostMidnightWrap(earlyA)).toBe(true);
        expect(ymd(tomorrowOf(wrapTargetDate(earlyA)))).toBe('2026-11-27');
    });

    it('rolls across month boundaries when opened after midnight on the 1st', () => {
        const decFirst = new Date(2026, 11, 1, 1, 15, 0);
        const target = wrapTargetDate(decFirst);

        expect(ymd(target)).toBe('2026-11-30');
        expect(ymd(tomorrowOf(target))).toBe('2026-12-01');
    });

    it('formats a readable header date', () => {
        expect(formatWrapDateLabel(new Date(2026, 10, 26, 0, 0, 0))).toBe('Thu, Nov 26');
    });
});
