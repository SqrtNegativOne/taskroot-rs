import { describe, expect, it } from 'vitest';
import { makeEvent } from '../../test/factories';
import {
    eventEndDateInclusive,
    eventIsAllDay,
    toAllDay,
    toTimed,
    withEndDate,
    withEndTime,
    withStartDate,
    withStartTime,
} from './eventDates';

function allDayEvent(startTime: string, endTime: string) {
    return makeEvent({ startTime, endTime, isAllDay: true });
}

function timedEvent(startTime: string, endTime: string) {
    return makeEvent({ startTime, endTime, isAllDay: false });
}

describe('eventIsAllDay', () => {
    it('trusts the flag when it is present', () => {
        expect(eventIsAllDay({ startTime: '2026-11-05T09:00:00', isAllDay: true })).toBe(true);
        expect(eventIsAllDay({ startTime: '2026-11-05', isAllDay: false })).toBe(false);
    });

    it('falls back to the date-only shape when the flag is absent', () => {
        expect(eventIsAllDay({ startTime: '2026-11-05' })).toBe(true);
        expect(eventIsAllDay({ startTime: '2026-11-05T09:00:00' })).toBe(false);
    });
});

describe('inclusive all-day ends', () => {
    it('shows the last day covered by an exclusive end', () => {
        expect(eventEndDateInclusive(allDayEvent('2026-11-05', '2026-11-07'))).toBe('2026-11-06');
    });

    it('treats a same-day exclusive end as a one-day event', () => {
        expect(eventEndDateInclusive(allDayEvent('2026-11-05', '2026-11-05'))).toBe('2026-11-05');
    });
});

describe('all-day and timed conversion', () => {
    it('round-trips an all-day event through timed and back', () => {
        const allDay = allDayEvent('2026-11-05', '2026-11-06');
        const timed = toTimed(allDay);
        expect(timed).toMatchObject({
            startTime: '2026-11-05T09:00:00',
            endTime: '2026-11-05T10:00:00',
            isAllDay: false,
        });

        const back = toAllDay(makeEvent({ ...timed }));
        expect(back).toEqual({
            startTime: '2026-11-05',
            endTime: '2026-11-06',
            isAllDay: true,
        });
    });

    it('keeps a multi-day timed event on its dates', () => {
        const timed = toTimed(allDayEvent('2026-11-05', '2026-11-08'));
        expect(timed.endTime.slice(0, 10)).toBe('2026-11-07');
    });

    it('keeps the covered days when a multi-day event becomes timed and back', () => {
        const timed = toTimed(allDayEvent('2026-11-05', '2026-11-08'));
        expect(timed.startTime).toBe('2026-11-05T09:00:00');
        expect(timed.endTime).toBe('2026-11-07T10:00:00');
        expect(toAllDay(makeEvent({ ...timed }))).toEqual({
            startTime: '2026-11-05',
            endTime: '2026-11-08',
            isAllDay: true,
        });
    });

    it('repairs a same-instant timed event when becoming timed', () => {
        const timed = toTimed(makeEvent({ startTime: '2026-11-05', endTime: '2026-11-05' }));
        expect(timed.endTime > timed.startTime).toBe(true);
    });
});

describe('moving start', () => {
    it('shifts the end by the same number of days', () => {
        const event = allDayEvent('2026-11-05', '2026-11-07');
        expect(withStartDate(event, '2026-11-08')).toEqual({
            startTime: '2026-11-08',
            endTime: '2026-11-10',
            isAllDay: true,
        });
    });

    it('keeps the duration when a timed start moves', () => {
        const event = timedEvent('2026-11-05T09:00:00', '2026-11-05T10:30:00');
        expect(withStartDate(event, '2026-11-07')).toEqual({
            startTime: '2026-11-07T09:00:00',
            endTime: '2026-11-07T10:30:00',
            isAllDay: false,
        });
    });

    it('moves the end forward when a start time passes it', () => {
        const event = timedEvent('2026-11-05T09:00:00', '2026-11-05T10:00:00');
        expect(withStartTime(event, '11:00')).toEqual({
            startTime: '2026-11-05T11:00:00',
            endTime: '2026-11-05T12:00:00',
            isAllDay: false,
        });
    });
});

describe('moving end', () => {
    it('stores the exclusive day after an inclusive all-day end', () => {
        const event = allDayEvent('2026-11-05', '2026-11-06');
        expect(withEndDate(event, '2026-11-09')).toEqual({
            startTime: '2026-11-05',
            endTime: '2026-11-10',
            isAllDay: true,
        });
    });

    it('collapses an all-day end before the start', () => {
        const event = allDayEvent('2026-11-05', '2026-11-06');
        expect(withEndDate(event, '2026-11-01').endTime).toBe('2026-11-06');
    });

    it('keeps a timed end on the same day when it still follows the start', () => {
        const event = timedEvent('2026-11-05T09:00:00', '2026-11-05T10:00:00');
        expect(withEndDate(event, '2026-11-05')).toEqual({
            startTime: '2026-11-05T09:00:00',
            endTime: '2026-11-05T10:00:00',
            isAllDay: false,
        });
    });


    it('rolls a timed end time to the next day when it would precede the start', () => {
        const event = timedEvent('2026-11-05T09:00:00', '2026-11-05T10:00:00');
        expect(withEndTime(event, '08:00')).toEqual({
            startTime: '2026-11-05T09:00:00',
            endTime: '2026-11-06T08:00:00',
            isAllDay: false,
        });
    });
});
