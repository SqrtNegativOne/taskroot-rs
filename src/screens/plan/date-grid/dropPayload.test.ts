import { describe, expect, it } from 'vitest';
import type { EventInstance } from '../../../lib/domain';
import { makeInstance, makeTaskDateInstance } from '../../../test/factories';
import { MS_PER_MINUTE, ymd } from '../../../lib/time';
import {
    DEFAULT_DROP_DURATION_MINUTES,
    gridDayDropPayload,
    movedTaskDateValue,
    snappedDropMinute,
    timelineDropPayload,
} from './dropPayload';
import { taskMarkerDropFor } from '../hooks/taskDrag.svelte';
import { MINUTES_PER_DAY } from '../snapping';
import { PX_PER_MIN, SNAP_MIN } from '../day-timeline/constants';

function timed(id: string, start: Date, end: Date): EventInstance {
    return makeInstance({
        id,
        timing: {
            kind: 'timed',
            start: start.toISOString(),
            end: end.toISOString(),
            timezone: null,
        },
    });
}

function allDay(id: string, startDate: string, endDateExclusive: string): EventInstance {
    return makeInstance({ id, timing: { kind: 'allDay', startDate, endDateExclusive } });
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

    it('keeps a longer-than-day duration inside the day instead of going negative', () => {
        expect(snappedDropMinute(pxFor(10 * 60), MINUTES_PER_DAY * 2)).toBe(0);
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

    it('clamps a longer-than-day instance to a full day', () => {
        const event = timed('t3', new Date(2026, 8, 7, 9, 0), new Date(2026, 8, 9, 10, 0));

        const payload = timelineDropPayload(event, '2026-09-10', 0);

        const start = new Date(payload.startTime);
        const end = new Date(payload.endTime);
        expect(end.getTime() - start.getTime()).toBe(MINUTES_PER_DAY * MS_PER_MINUTE);
    });
});

describe('movedTaskDateValue & taskMarkerDropFor', () => {
    it('moves a date-only marker to the target YYYY-MM-DD and ignores same-day drops', () => {
        const timing = { kind: 'allDay' as const, date: '2026-09-07' };
        expect(movedTaskDateValue(timing, '2026-09-10')).toBe('2026-09-10');
        expect(movedTaskDateValue(timing, '2026-09-07')).toBeUndefined();
    });

    it('preserves local HH:MM:00 when moving a timed marker to a new day', () => {
        const local = new Date(2026, 8, 7, 9, 45);
        const timing = { kind: 'timed' as const, start: local.toISOString(), timezone: null };

        expect(movedTaskDateValue(timing, '2026-09-11')).toBe('2026-09-11T09:45:00');
        expect(movedTaskDateValue(timing, '2026-09-07')).toBeUndefined();
    });

    it('routes grid drops to the marker field and timeline drops to scheduling', () => {
        const startMarker = makeTaskDateInstance({
            taskId: 't1',
            kind: 'start',
            timing: { kind: 'allDay', date: '2026-09-07' },
        });
        const dueMarker = makeTaskDateInstance({
            taskId: 't1',
            kind: 'due',
            timing: { kind: 'timed', start: new Date(2026, 8, 7, 14, 15).toISOString(), timezone: null },
        });

        expect(taskMarkerDropFor(startMarker, { kind: 'grid-day', date: '2026-09-09' }, 30)).toEqual({
            kind: 'move',
            field: 'start',
            value: '2026-09-09',
        });
        expect(taskMarkerDropFor(startMarker, { kind: 'grid-day', date: '2026-09-07' }, 30)).toBeUndefined();
        expect(taskMarkerDropFor(dueMarker, { kind: 'grid-day', date: '2026-09-10' }, 30)).toEqual({
            kind: 'move',
            field: 'due',
            value: '2026-09-10T14:15:00',
        });
        expect(
            taskMarkerDropFor(dueMarker, { kind: 'day-time', date: '2026-09-12', minute: 600 }, 45),
        ).toEqual({
            kind: 'schedule',
            slot: { date: '2026-09-12', minute: 600, duration: 45 },
        });
    });
});
