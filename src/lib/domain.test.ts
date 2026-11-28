import { describe, expect, it } from 'vitest';
import type { AppCalendar } from './domain';
import {
    buildNewEvent,
    computeFilterDefaults,
    filterGroup,
    isEventReadOnly,
    resolveDefaultCalendar,
} from './domain';

describe('computeFilterDefaults', () => {
    it('returns empty object when no filters are present', () => {
        expect(computeFilterDefaults()).toEqual({});
    });

    it('infers status and priority from "is" filters', () => {
        const filters = [
            { column: 'status', operator: 'is', value: 'done' },
            { column: 'priority', operator: 'is', value: 3 },
        ];
        expect(computeFilterDefaults(filterGroup(filters))).toEqual({ status: 'done', priority: 3 });
    });

    it('picks next fallback status when statuses are excluded', () => {
        expect(computeFilterDefaults(filterGroup([{ column: 'status', operator: 'is not', value: 'todo' }]))).toEqual({
            status: 'next-up',
        });
        expect(
            computeFilterDefaults(
                filterGroup([{ column: 'status', operator: 'is not', value: ['todo', 'next-up', 'done'] }]),
            ),
        ).toEqual({ status: 'cancelled' });
    });

    it('does not set field if required value is also excluded', () => {
        const filters = [
            { column: 'status', operator: 'is', value: 'done' },
            { column: 'status', operator: 'is not', value: 'done' },
        ];
        expect(computeFilterDefaults(filterGroup(filters))).toEqual({});
    });

    it('includes valid tags and strips excluded tags', () => {
        const filters = [
            { column: 'tags', operator: 'is', value: ['feature', 'bug'] },
            { column: 'tags', operator: 'is not', value: 'bug' },
        ];
        expect(computeFilterDefaults(filterGroup(filters))).toEqual({ tags: ['feature'] });
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

    it('does not inherit read-only status from calendars[0] when the event calendar is unknown', () => {
        const readOnlyFirst: AppCalendar[] = [
            { id: 'cal-holidays', summary: 'Holidays', writable: false, component: 'VEVENT' },
            { id: 'cal-work', summary: 'Work', writable: true, component: 'VEVENT' },
        ];
        expect(isEventReadOnly({ remoteCollectionId: 'cal-missing' }, readOnlyFirst)).toBe(false);
        expect(isEventReadOnly({ calendarId: 'cal-missing' }, readOnlyFirst)).toBe(false);
    });
});

describe('resolveDefaultCalendar', () => {
    const calendars: AppCalendar[] = [
        { id: 'cal-ro-events', summary: 'Holidays', writable: false, component: 'VEVENT' },
        { id: 'cal-events-1', summary: 'Personal', writable: true, component: 'VEVENT' },
        { id: 'cal-events-2', summary: 'Work', writable: true, component: 'VEVENT' },
        { id: 'cal-ro-tasks', summary: 'Archive', writable: false, component: 'VTODO' },
        { id: 'cal-tasks-1', summary: 'Inbox', writable: true, component: 'VTODO' },
        { id: 'cal-tasks-2', summary: 'Projects', writable: true, component: 'VTODO' },
    ];

    it('prefers the configured default_event_list or default_task_list when writable and matching', () => {
        expect(resolveDefaultCalendar(calendars, 'VEVENT', 'cal-events-2')?.id).toBe('cal-events-2');
        expect(resolveDefaultCalendar(calendars, 'VTODO', 'cal-tasks-2')?.id).toBe('cal-tasks-2');
    });

    it('falls back to the first writable calendar of the component when preferred is unset, missing, read-only, or wrong component', () => {
        expect(resolveDefaultCalendar(calendars, 'VEVENT')?.id).toBe('cal-events-1');
        expect(resolveDefaultCalendar(calendars, 'VEVENT', '')?.id).toBe('cal-events-1');
        expect(resolveDefaultCalendar(calendars, 'VEVENT', 'cal-missing')?.id).toBe('cal-events-1');
        expect(resolveDefaultCalendar(calendars, 'VEVENT', 'cal-ro-events')?.id).toBe('cal-events-1');
        expect(resolveDefaultCalendar(calendars, 'VEVENT', 'cal-tasks-2')?.id).toBe('cal-events-1');

        expect(resolveDefaultCalendar(calendars, 'VTODO')?.id).toBe('cal-tasks-1');
        expect(resolveDefaultCalendar(calendars, 'VTODO', '')?.id).toBe('cal-tasks-1');
        expect(resolveDefaultCalendar(calendars, 'VTODO', 'cal-ro-tasks')?.id).toBe('cal-tasks-1');
        expect(resolveDefaultCalendar(calendars, 'VTODO', 'cal-events-2')?.id).toBe('cal-tasks-1');
    });

    it('returns undefined when no writable calendar exists for the component', () => {
        const readOnlyOnly = calendars.filter((calendar) => !calendar.writable);
        expect(resolveDefaultCalendar(readOnlyOnly, 'VTODO', 'cal-ro-tasks')).toBeUndefined();
        expect(resolveDefaultCalendar(readOnlyOnly, 'VEVENT', 'cal-ro-events')).toBeUndefined();
        expect(resolveDefaultCalendar([], 'VTODO')).toBeUndefined();
    });
});

describe('buildNewEvent', () => {
    const date = new Date(2026, 8, 3, 15, 0);

    it('creates a floating all-day event and leaves remoteCollectionId unset for Rust default resolution', () => {
        const event = buildNewEvent(date);

        expect(event.title).toBe('New Event');
        expect(typeof event.id).toBe('string');
        expect(event.startTime).toBe('2026-09-03');
        expect(event.endTime).toBe('2026-09-04');
        expect(event.isAllDay).toBe(true);
        expect(event.timezone).toBeUndefined();
        expect(event.remoteCollectionId).toBeUndefined();
    });

    it('creates a timed event with canonical local wall-clock strings in the system timezone and leaves remoteCollectionId unset', () => {
        const event = buildNewEvent(date, {
            startMinutes: 10 * 60,
            endMinutes: 11 * 60 + 30,
        });

        expect(event.startTime).toBe('2026-09-03T10:00:00');
        expect(event.endTime).toBe('2026-09-03T11:30:00');
        expect(event.isAllDay).toBe(false);
        expect(event.timezone).toBe(Intl.DateTimeFormat().resolvedOptions().timeZone);
        expect(event.remoteCollectionId).toBeUndefined();
    });

    it('defaults a timed event to one hour when no end is given', () => {
        const event = buildNewEvent(date, { startMinutes: 9 * 60 });

        expect(event.startTime).toBe('2026-09-03T09:00:00');
        expect(event.endTime).toBe('2026-09-03T10:00:00');
        expect(event.remoteCollectionId).toBeUndefined();
    });

    it('adopts the dragged task title and link', () => {
        const event = buildNewEvent(date, {
            startMinutes: 9 * 60,
            endMinutes: 9 * 60 + 30,
            title: 'Write the report',
            taskId: 'task-42',
        });

        expect(event.title).toBe('Write the report');
        expect(event.taskId).toBe('task-42');
        expect(event.remoteCollectionId).toBeUndefined();
    });
});
