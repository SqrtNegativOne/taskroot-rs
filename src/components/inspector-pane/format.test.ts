import { describe, expect, it } from 'vitest';
import { withDate, withTime } from './format';

describe('withDate', () => {
    it('keeps a timed value timed when its date changes', () => {
        expect(withDate('2026-10-27T09:15:00', '2026-10-28')).toBe('2026-10-28T09:15:00');
    });

    it('keeps a date-only value date-only', () => {
        expect(withDate('2026-10-27', '2026-10-28')).toBe('2026-10-28');
    });

    it('clears the value for an empty date', () => {
        expect(withDate('2026-10-27T09:15:00', '')).toBeUndefined();
    });
});

describe('withTime', () => {
    it('adds a time to a date-only value', () => {
        expect(withTime('2026-10-27', '09:15')).toBe('2026-10-27T09:15:00');
    });

    it('replaces only the time part', () => {
        expect(withTime('2026-10-27T08:00:00', '09:15')).toBe('2026-10-27T09:15:00');
    });

    it('clears the time back to a date-only value', () => {
        expect(withTime('2026-10-27T09:15:00', '')).toBe('2026-10-27');
    });

    it('has nothing to edit without a date', () => {
        expect(withTime(undefined, '09:15')).toBeUndefined();
    });
});
