import { describe, expect, it } from 'vitest';
import type { EventInstance } from '../bindings/EventInstance.generated';
import { instanceOccursOnDay } from './timing';

// Pin the host zone so the DST edge cases below exercise real transitions on
// any machine: 2026-11-01 is Chicago's 25-hour fall-back day, 2026-03-08 its
// 23-hour spring-forward day.
process.env.TZ = 'America/Chicago';

function allDay(startDate: string, endDateExclusive: string): EventInstance {
    return {
        id: 'e1',
        occurrenceKey: `e1@${startDate}`,
        occurrenceStart: startDate,
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
        occurrenceStart: startIso,
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

    it('does not let a 25-hour fall-back day claim the next local day\'s early events', () => {
        // 2026-11-01 runs 25 hours, so a +24h day end lands at 23:00 on
        // 2026-11-02 and would wrongly include this event in the previous day.
        const start = new Date(2026, 10, 2, 0, 30);
        const end = new Date(2026, 10, 2, 1, 0);
        const call = timed(start.toISOString(), end.toISOString());

        expect(instanceOccursOnDay(call, new Date(2026, 10, 1))).toBe(false);
        expect(instanceOccursOnDay(call, new Date(2026, 10, 2))).toBe(true);
    });

    it('does not let a 23-hour spring-forward day claim the next local day\'s early events', () => {
        // 2026-03-08 runs 23 hours, so a +24h day end lands at 01:00 on
        // 2026-03-09 and would wrongly include this event in the previous day.
        const start = new Date(2026, 2, 9, 0, 30);
        const end = new Date(2026, 2, 9, 1, 0);
        const call = timed(start.toISOString(), end.toISOString());

        expect(instanceOccursOnDay(call, new Date(2026, 2, 8))).toBe(false);
        expect(instanceOccursOnDay(call, new Date(2026, 2, 9))).toBe(true);
    });
});
