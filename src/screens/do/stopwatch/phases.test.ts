import { describe, expect, it } from 'vitest';
import { guzeyPhase, guzeyRemainingMs } from './phases';

describe('guzeyPhase', () => {
    it('is work outside the break windows', () => {
        expect(guzeyPhase(10, 10, false)).toBe('work');
        expect(guzeyPhase(10, 40, false)).toBe('work');
    });

    it('takes short breaks at the top and middle of each half hour', () => {
        expect(guzeyPhase(10, 0, false)).toBe('break');
        expect(guzeyPhase(10, 4, false)).toBe('break');
        expect(guzeyPhase(10, 30, false)).toBe('break');
        expect(guzeyPhase(10, 34, false)).toBe('break');
    });

    it('ignores the long-break window when long breaks are disabled', () => {
        expect(guzeyPhase(3, 10, false)).toBe('work');
    });

    it('takes a long break at the start of every third hour when enabled', () => {
        expect(guzeyPhase(3, 0, true)).toBe('long break');
        expect(guzeyPhase(3, 34, true)).toBe('long break');
        expect(guzeyPhase(6, 10, true)).toBe('long break');
    });

    it('returns to work after the long break and outside third hours', () => {
        expect(guzeyPhase(3, 35, true)).toBe('work');
        expect(guzeyPhase(4, 10, true)).toBe('work');
    });
});

describe('guzeyRemainingMs', () => {
    it('counts down to the end of the first short break', () => {
        expect(guzeyRemainingMs(10, 3, 0, 0, false)).toBe(2 * 60_000);
    });

    it('counts down to the end of a work block', () => {
        expect(guzeyRemainingMs(10, 12, 30, 0, false)).toBe(17 * 60_000 + 30_000);
    });

    it('counts down to the end of the half-hour break', () => {
        expect(guzeyRemainingMs(10, 32, 0, 0, false)).toBe(3 * 60_000);
    });

    it('counts down to the top of the hour during the second work block', () => {
        expect(guzeyRemainingMs(10, 50, 0, 0, false)).toBe(10 * 60_000);
    });

    it('uses the long-break end while a long break is active', () => {
        expect(guzeyRemainingMs(3, 33, 0, 0, true)).toBe(2 * 60_000);
    });

    it('treats the long-break window as a short break when disabled', () => {
        expect(guzeyRemainingMs(3, 3, 0, 0, false)).toBe(2 * 60_000);
    });
});
