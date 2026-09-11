import { describe, expect, it } from 'vitest';
import type { EventInstance } from '../bindings/EventInstance.generated';
import { instanceOccursOnDay, isAllDayTiming } from './timing';

function allDay(startDate: string, endDateExclusive: string): EventInstance {
    return {
        id: 'e1',
        occurrenceKey: `e1@${startDate}`,
        title: 'Trip',
        timing: { kind: 'allDay', startDate, endDateExclusive },
        recurring: false,
        status: 'confirmed',
    };
}

function timed(startIso: string, endIso: string): EventInstance {
    return {
        id: 'e2',
        occurrenceKey: `e2@${startIso}`,
        title: 'Call',
        timing: { kind: 'timed', start: startIso, end: endIso, timezone: null },
        recurring: false,
        status: 'confirmed',
    };
}

describe('instanceOccursOnDay', () => {
    it('spreads an all-day event across every floating date in its half-open span', () => {
        const trip = allDay('2026-09-07', '2026-09-10');

        expect(instanceOccursOnDay(trip, new Date(2026, 8, 6))).toBe(false);
        expect(instanceOccursOnDay(trip, new Date(2026, 8, 7))).toBe(true);
        expect(instanceOccursOnDay(trip, new Date(2026, 8, 8))).toBe(true);
        expect(instanceOccursOnDay(trip, new Date(2026, 8, 9))).toBe(true);
        expect(instanceOccursOnDay(trip, new Date(2026, 8, 10))).toBe(false);
    });

    it('ignores the host timezone for floating all-day dates', () => {
        // A date string is compared lexically, so a negative-offset host can
        // never shift the event onto the previous local day.
        const holiday = allDay('2026-01-01', '2026-01-02');
        expect(instanceOccursOnDay(holiday, new Date(2026, 0, 1))).toBe(true);
    });

    it('buckets a timed event by local day across midnight', () => {
        const start = new Date(2026, 8, 7, 22, 0);
        const end = new Date(2026, 8, 8, 2, 0);
        const call = timed(start.toISOString(), end.toISOString());

        expect(instanceOccursOnDay(call, new Date(2026, 8, 7))).toBe(true);
        expect(instanceOccursOnDay(call, new Date(2026, 8, 8))).toBe(true);
        expect(instanceOccursOnDay(call, new Date(2026, 8, 9))).toBe(false);
    });

    it('discriminates timing variants', () => {
        expect(isAllDayTiming(allDay('2026-09-07', '2026-09-08').timing)).toBe(true);
        expect(isAllDayTiming(timed('2026-09-07T09:00:00Z', '2026-09-07T10:00:00Z').timing)).toBe(
            false,
        );
    });
});
