import { describe, expect, it } from 'vitest';
import type { EventInstance } from '../../../lib/domain';
import { ymd } from '../../../lib/time';
import { bucketEventsByDay } from './bucketing';

const MINUTES_IN_HOUR = 60;
const HOURS_PER_DAY = 24;
const MINUTES_PER_DAY = HOURS_PER_DAY * MINUTES_IN_HOUR;

function timed(id: string, start: Date, end: Date): EventInstance {
    return {
        id,
        occurrenceKey: `${id}@${start.toISOString()}`,
        title: 'Event',
        timing: {
            kind: 'timed',
            start: start.toISOString(),
            end: end.toISOString(),
            timezone: null,
        },
        recurring: false,
        status: 'confirmed',
    };
}

function allDay(id: string, startDate: string, endDateExclusive: string): EventInstance {
    return {
        id,
        occurrenceKey: `${id}@${startDate}`,
        title: 'Event',
        timing: { kind: 'allDay', startDate, endDateExclusive },
        recurring: false,
        status: 'confirmed',
    };
}

describe('bucketEventsByDay', () => {
    it('buckets an early local-morning event to its local day, not its UTC date', () => {
        const monday = new Date(2026, 8, 7);
        const start = new Date(2026, 8, 7, 0, 30);
        const end = new Date(2026, 8, 7, 1, 0);

        const bucketed = bucketEventsByDay([timed('e1', start, end)], [monday]);

        const laid = bucketed[ymd(monday)] ?? [];
        expect(laid.map((e) => [e.startMins, e.endMins])).toEqual([[30, 60]]);
    });

    it('leaves all-day events to the date grid', () => {
        const monday = new Date(2026, 8, 7);

        const bucketed = bucketEventsByDay([allDay('e1', '2026-09-07', '2026-09-08')], [monday]);

        expect(bucketed[ymd(monday)]).toBeUndefined();
    });

    it('clips a multi-day event to each day it overlaps', () => {
        const day1 = new Date(2026, 8, 7);
        const day2 = new Date(2026, 8, 8);
        const day3 = new Date(2026, 8, 9);
        const start = new Date(2026, 8, 7, 22, 0);
        const end = new Date(2026, 8, 9, 2, 0);

        const bucketed = bucketEventsByDay([timed('e1', start, end)], [day1, day2, day3]);

        expect(bucketed[ymd(day1)].map((e) => [e.startMins, e.endMins])).toEqual([
            [22 * MINUTES_IN_HOUR, MINUTES_PER_DAY],
        ]);
        expect(bucketed[ymd(day2)].map((e) => [e.startMins, e.endMins])).toEqual([
            [0, MINUTES_PER_DAY],
        ]);
        expect(bucketed[ymd(day3)].map((e) => [e.startMins, e.endMins])).toEqual([
            [0, 2 * MINUTES_IN_HOUR],
        ]);
    });

    it('drops events that do not overlap the requested days', () => {
        const monday = new Date(2026, 8, 7);
        const start = new Date(2026, 8, 10, 9, 0);
        const end = new Date(2026, 8, 10, 10, 0);

        const bucketed = bucketEventsByDay([timed('e1', start, end)], [monday]);

        expect(Object.keys(bucketed)).toHaveLength(0);
    });

    it('buckets two same-day occurrences of one master independently', () => {
        const monday = new Date(2026, 8, 7);
        const first = timed('master', new Date(2026, 8, 7, 9, 0), new Date(2026, 8, 7, 9, 30));
        const second = timed('master', new Date(2026, 8, 7, 15, 0), new Date(2026, 8, 7, 15, 30));

        const bucketed = bucketEventsByDay([first, second], [monday]);

        expect(bucketed[ymd(monday)]).toHaveLength(2);
    });
});
