import { describe, expect, it } from 'vitest';
import type { EventInstance } from '../../../lib/domain';
import { ymd } from '../../../lib/time';
import {
    DEFAULT_DROP_DURATION_MINUTES,
    gridDayDropPayload,
    snappedDropMinute,
    timelineDropPayload,
} from './dropPayload';
import { PX_PER_MIN, SNAP_MIN } from '../day-timeline/constants';

const MS_PER_MINUTE = 60_000;

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

describe('snappedDropMinute', () => {
    const pxFor = (minutes: number) => minutes * PX_PER_MIN;

    it('snaps a pixel offset to the nearest step', () => {
        expect(snappedDropMinute(pxFor(7), 60)).toBe(0);
        expect(snappedDropMinute(pxFor(8), 60)).toBe(SNAP_MIN);
        expect(snappedDropMinute(pxFor(37), 60)).toBe(30);
    });

    it('clamps a drop so the block stays inside the day', () => {
        expect(snappedDropMinute(pxFor(23 * 60), 60)).toBe(23 * 60);
        expect(snappedDropMinute(pxFor(-30), 60)).toBe(0);
    });
});

describe('gridDayDropPayload', () => {
    it('keeps the local time-of-day and duration of a timed instance', () => {
        const event = timed('t1', new Date(2026, 8, 7, 9, 30), new Date(2026, 8, 7, 11, 0));

        const payload = gridDayDropPayload(event, '2026-09-10');

        const start = new Date(payload.startTime);
        const end = new Date(payload.endTime);
        expect(payload.isAllDay).toBe(false);
        expect(ymd(start)).toBe('2026-09-10');
        expect(start.getHours()).toBe(9);
        expect(start.getMinutes()).toBe(30);
        expect(end.getTime() - start.getTime()).toBe(90 * MS_PER_MINUTE);
    });

    it('shifts an all-day instance by whole days without making it timed', () => {
        const event = allDay('a', '2026-09-07', '2026-09-09');

        expect(gridDayDropPayload(event, '2026-09-14')).toEqual({
            startTime: '2026-09-14',
            endTime: '2026-09-16',
            isAllDay: true,
        });
    });
});

describe('timelineDropPayload', () => {
    it('preserves the duration of a timed instance', () => {
        const event = timed('t2', new Date(2026, 8, 7, 9, 30), new Date(2026, 8, 7, 10, 0));

        const payload = timelineDropPayload(event, '2026-09-10', 14 * 60 + 15);

        const start = new Date(payload.startTime);
        const end = new Date(payload.endTime);
        expect(payload.isAllDay).toBe(false);
        expect(ymd(start)).toBe('2026-09-10');
        expect(start.getHours()).toBe(14);
        expect(start.getMinutes()).toBe(15);
        expect(end.getTime() - start.getTime()).toBe(30 * MS_PER_MINUTE);
    });

    it('turns an all-day instance into a default one-hour timed block', () => {
        const event = allDay('a', '2026-09-07', '2026-09-08');

        const payload = timelineDropPayload(event, '2026-09-10', 10 * 60);

        const start = new Date(payload.startTime);
        const end = new Date(payload.endTime);
        expect(payload.isAllDay).toBe(false);
        expect(start.getHours()).toBe(10);
        expect(end.getTime() - start.getTime()).toBe(
            DEFAULT_DROP_DURATION_MINUTES * MS_PER_MINUTE,
        );
    });
});
