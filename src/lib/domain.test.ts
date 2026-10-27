import { describe, expect, it } from 'vitest';
import type { AppCalendar } from './domain';
import {
    buildNewEvent,
    computeFilterDefaults,
    isAppTaskStatus,
    isCalendarReadOnly,
    isEventReadOnly,
} from './domain';

describe('isAppTaskStatus', () => {
    it('accepts valid task statuses', () => {
        expect(isAppTaskStatus('todo')).toBe(true);
        expect(isAppTaskStatus('next-up')).toBe(true);
        expect(isAppTaskStatus('doing')).toBe(true);
        expect(isAppTaskStatus('done')).toBe(true);
    });

    it('rejects invalid or non-string values', () => {
        expect(isAppTaskStatus('in-progress')).toBe(false);
        expect(isAppTaskStatus('completed')).toBe(false);
        expect(isAppTaskStatus('TODO')).toBe(false);
        expect(isAppTaskStatus('')).toBe(false);
        expect(isAppTaskStatus(null)).toBe(false);
        expect(isAppTaskStatus(undefined)).toBe(false);
    });
});

describe('computeFilterDefaults', () => {
    it('returns empty object when no filters are present', () => {
        expect(computeFilterDefaults([])).toEqual({});
    });

    it('infers status and priority from "is" filters', () => {
        const filters = [
            { column: 'status', operator: 'is', value: 'done' },
            { column: 'priority', operator: 'is', value: 3 },
        ];
        expect(computeFilterDefaults(filters)).toEqual({ status: 'done', priority: 3 });
    });

    it('picks next fallback status when status is excluded', () => {
        const filters = [{ column: 'status', operator: 'is not', value: 'todo' }];
        expect(computeFilterDefaults(filters)).toEqual({ status: 'next-up' });
    });

    it('picks fallback after multiple excluded statuses', () => {
        const filters = [
            { column: 'status', operator: 'is not', value: ['todo', 'next-up', 'doing'] },
        ];
        expect(computeFilterDefaults(filters)).toEqual({ status: 'done' });
    });

    it('does not set field if required value is also excluded', () => {
        const filters = [
            { column: 'status', operator: 'is', value: 'done' },
            { column: 'status', operator: 'is not', value: 'done' },
        ];
        expect(computeFilterDefaults(filters)).toEqual({});
    });

    it('includes valid tags and strips excluded tags', () => {
        const filters = [
            { column: 'tag', operator: 'is', value: ['feature', 'bug'] },
            { column: 'tags', operator: 'is not', value: 'bug' },
        ];
        expect(computeFilterDefaults(filters)).toEqual({ tags: ['feature'] });
    });
});

describe('isCalendarReadOnly', () => {
    it('treats a non-writable calendar as read-only', () => {
        expect(isCalendarReadOnly({ writable: false })).toBe(true);
    });

    it('treats writable and unset calendars as editable', () => {
        expect(isCalendarReadOnly({ writable: true })).toBe(false);
        expect(isCalendarReadOnly({ writable: undefined })).toBe(false);
        expect(isCalendarReadOnly(undefined)).toBe(false);
    });
});

describe('isEventReadOnly', () => {
    const calendars: AppCalendar[] = [
        { id: 'cal-work', summary: 'Work', writable: true, component: 'VEVENT' },
        { id: 'cal-holidays', summary: 'Holidays', writable: false, component: 'VEVENT' },
        { id: 'cal-busy', summary: 'Busy', writable: false, component: 'VEVENT' },
    ];

    it('checks the calendar the event belongs to', () => {
        expect(isEventReadOnly({ remoteCollectionId: 'cal-work' }, calendars)).toBe(false);
        expect(isEventReadOnly({ remoteCollectionId: 'cal-holidays' }, calendars)).toBe(true);
        expect(isEventReadOnly({ calendarId: 'cal-busy' }, calendars)).toBe(true);
    });

    it('leaves events without a calendar editable', () => {
        expect(isEventReadOnly({}, calendars)).toBe(false);
        expect(isEventReadOnly({ remoteCollectionId: undefined }, [])).toBe(false);
    });
});

describe('buildNewEvent', () => {
    const date = new Date(2026, 8, 3, 15, 0);
    const calendars: AppCalendar[] = [
        { id: 'cal-tasks', summary: 'Tasks', writable: true, component: 'VTODO' },
        { id: 'cal-readonly', summary: 'Holidays', writable: false, component: 'VEVENT' },
        { id: 'cal-events', summary: 'Events', writable: true, component: 'VEVENT' },
    ];

    it('creates a floating all-day event and targets a writable VEVENT calendar', () => {
        const event = buildNewEvent(date, { calendars });

        expect(event.title).toBe('New Event');
        expect(typeof event.id).toBe('string');
        expect(event.startTime).toBe('2026-09-03');
        expect(event.endTime).toBe('2026-09-04');
        expect(event.isAllDay).toBe(true);
        expect(event.timezone).toBeUndefined();
        expect(event.remoteCollectionId).toBe('cal-events');
    });

    it('creates a timed event with UTC instants in the system timezone', () => {
        const event = buildNewEvent(date, {
            startMinutes: 10 * 60,
            endMinutes: 11 * 60 + 30,
            calendars,
        });

        expect(event.startTime).toBe(new Date(2026, 8, 3, 10, 0).toISOString());
        expect(event.endTime).toBe(new Date(2026, 8, 3, 11, 30).toISOString());
        expect(event.isAllDay).toBe(false);
        expect(event.timezone).toBe(Intl.DateTimeFormat().resolvedOptions().timeZone);
    });

    it('defaults a timed event to one hour when no end is given', () => {
        const event = buildNewEvent(date, { startMinutes: 9 * 60 });

        expect(event.startTime).toBe(new Date(2026, 8, 3, 9, 0).toISOString());
        expect(event.endTime).toBe(new Date(2026, 8, 3, 10, 0).toISOString());
    });

    it('leaves the collection unset when no writable event calendar exists', () => {
        const taskCalendars = calendars.filter((calendar) => calendar.component === 'VTODO');
        const event = buildNewEvent(date, { calendars: taskCalendars });

        expect(event.remoteCollectionId).toBeUndefined();
    });

    it('adopts the dragged task title and link', () => {
        const event = buildNewEvent(date, {
            startMinutes: 9 * 60,
            endMinutes: 9 * 60 + 30,
            calendars,
            title: 'Write the report',
            taskId: 'task-42',
        });

        expect(event.title).toBe('Write the report');
        expect(event.taskId).toBe('task-42');
    });
});
