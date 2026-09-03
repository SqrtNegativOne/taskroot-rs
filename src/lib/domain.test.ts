import { describe, expect, it } from 'vitest';
import type { AppCalendar, AppEvent, AppTask } from './domain';
import {
    computeFilterDefaults,
    editing,
    filterEvents,
    hydrateEvents,
    isAppTaskStatus,
    isEventAllDay,
    isYmdString,
    sortEvents,
    toEventType,
} from './domain';

describe('isYmdString', () => {
    it('returns true for valid YYYY-MM-DD strings', () => {
        expect(isYmdString('2026-09-03')).toBe(true);
        expect(isYmdString('1999-12-31')).toBe(true);
    });

    it('returns false for invalid formats or non-strings', () => {
        expect(isYmdString('2026-9-3')).toBe(false);
        expect(isYmdString('2026/09/03')).toBe(false);
        expect(isYmdString('2026-09-03T10:00:00')).toBe(false);
        expect(isYmdString('')).toBe(false);
        expect(isYmdString(null)).toBe(false);
        expect(isYmdString(undefined)).toBe(false);
        expect(isYmdString(20260903)).toBe(false);
    });
});

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

describe('toEventType', () => {
    it('returns known event types verbatim', () => {
        expect(toEventType('busy', 'info')).toBe('busy');
        expect(toEventType('info', 'busy')).toBe('info');
        expect(toEventType('log', 'busy')).toBe('log');
    });

    it('returns fallback for unknown or missing raw types', () => {
        expect(toEventType('meeting', 'busy')).toBe('busy');
        expect(toEventType(undefined, 'info')).toBe('info');
        expect(toEventType('', 'log')).toBe('log');
    });
});

describe('editing builder', () => {
    const original: AppTask = {
        id: 'task-1',
        title: 'Original Title',
        status: 'todo',
        notes: 'Initial notes',
    };

    it('updates properties immutably via .set()', () => {
        const updated = editing(original)
            .set('title', 'Updated Title')
            .set('status', 'doing')
            .done();

        expect(updated.title).toBe('Updated Title');
        expect(updated.status).toBe('doing');
        expect(original.title).toBe('Original Title');
        expect(original.status).toBe('todo');
    });

    it('removes optional properties completely via .clear()', () => {
        const updated = editing(original).clear('notes').done();

        expect('notes' in updated).toBe(false);
        expect(original.notes).toBe('Initial notes');
    });
});

describe('hydrateEvents', () => {
    const sampleTasks: AppTask[] = [
        { id: 't1', title: 'Write tests', status: 'next-up', priority: 1 },
        { id: 't2', title: 'Ship feature', status: 'done', priority: 2 },
    ];

    const sampleCalendars: AppCalendar[] = [
        { id: 'cal-work', summary: 'Work Calendar', color: '#4285f4', isPrimary: true },
        { id: 'cal-home', summary: 'Home Life', color: '#0f9d58' },
    ];

    it('hydrates event linked to a task with task details and calendar metadata', () => {
        const events: AppEvent[] = [
            {
                id: 'e1',
                title: 'Placeholder',
                startTime: '2026-09-03T10:00:00',
                endTime: '2026-09-03T11:00:00',
                taskId: 't1',
                remoteCollectionId: 'cal-work',
            },
        ];

        const [hydrated] = hydrateEvents(events, sampleTasks, sampleCalendars);

        expect(hydrated.title).toBe('Write tests');
        expect(hydrated.task?.id).toBe('t1');
        expect(hydrated.task?.status).toBe('next-up');
        expect(hydrated.color).toBe('#4285f4');
        expect(hydrated.category).toBe('Work Calendar');
    });

    it('keeps original title when event has no taskId', () => {
        const events: AppEvent[] = [
            {
                id: 'e2',
                title: 'Lunch break',
                startTime: '2026-09-03T12:00:00',
                endTime: '2026-09-03T13:00:00',
                remoteCollectionId: 'cal-home',
            },
        ];

        const [hydrated] = hydrateEvents(events, sampleTasks, sampleCalendars);

        expect(hydrated.title).toBe('Lunch break');
        expect(hydrated.task).toBeUndefined();
        expect(hydrated.category).toBe('Home Life');
    });

    it('falls back to primary calendar when collection ID is primary or omitted', () => {
        const events: AppEvent[] = [
            {
                id: 'e3',
                title: 'Unsynced Event',
                startTime: '2026-09-03T14:00:00',
                endTime: '2026-09-03T15:00:00',
            },
        ];

        const [hydrated] = hydrateEvents(events, [], sampleCalendars);

        expect(hydrated.category).toBe('Work Calendar');
        expect(hydrated.color).toBe('#4285f4');
    });

    it('safely handles empty calendar and task lists', () => {
        const events: AppEvent[] = [
            {
                id: 'e4',
                title: 'Solo Event',
                startTime: '2026-09-03T09:00:00',
                endTime: '2026-09-03T10:00:00',
            },
        ];

        const [hydrated] = hydrateEvents(events, [], []);

        expect(hydrated.title).toBe('Solo Event');
        expect(hydrated.category).toBeUndefined();
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

describe('event helpers: isEventAllDay, filterEvents, sortEvents', () => {
    it('identifies all-day events correctly', () => {
        expect(isEventAllDay({ startTime: '2026-09-03', endTime: '2026-09-03' })).toBe(true);
        expect(isEventAllDay({ startTime: '2026-09-03T10:00:00', endTime: '2026-09-03T11:00:00' })).toBe(false);
        expect(isEventAllDay({ startTime: '2026-09-03', endTime: '2026-09-03', isAllDay: false })).toBe(false);
    });

    it('filters and sorts hydrated events', () => {
        const evs = [
            { id: 'e2', title: 'Beta', startTime: '2026-09-03T11:00:00', endTime: '2026-09-03T12:00:00', category: 'Work' },
            { id: 'e1', title: 'Alpha', startTime: '2026-09-03T09:00:00', endTime: '2026-09-03T10:00:00', category: 'Personal' },
        ];

        const filtered = filterEvents(evs, [{ column: 'category', operator: 'is', value: 'Work' }]);
        expect(filtered.map((e) => e.id)).toEqual(['e2']);

        const sorted = sortEvents(evs, 'startTime');
        expect(sorted.map((e) => e.id)).toEqual(['e1', 'e2']);
    });
});
