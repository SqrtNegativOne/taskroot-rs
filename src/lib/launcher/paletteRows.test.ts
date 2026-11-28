import { describe, expect, it } from 'vitest';
import type { AppCalendar, AppTask, EventInstance } from '../domain';
import { makeInstance } from '../../test/factories';
import { Routes } from '../routes';
import { parseQuickCapture } from './quickCapture';
import {
    buildPaletteRows,
    buildTaskFromQuickCapture,
    filterTodayTasks,
    formatEventDetail,
    formatPreviewParts,
    formatTaskDetail,
    matchRouteCommand,
} from './paletteRows';

const SAMPLE_CALENDARS: AppCalendar[] = [
    { id: '/tasks/', summary: 'Tasks', writable: true, component: 'VTODO' },
    { id: '/work/', summary: 'Work', writable: true, component: 'VTODO' },
];

const TODAY = new Date(2026, 10, 19, 10, 0, 0);

function makeEvent(id: string, title: string, timing: EventInstance['timing']): EventInstance {
    return makeInstance({
        id,
        occurrenceKey: `${id}:0`,
        title,
        timing,
        calendarId: '/events/',
    });
}

describe('paletteRows', () => {
    it('matches route commands case-insensitively with trimming', () => {
        expect(matchRouteCommand('  Settings ')).toBe(Routes.SETTINGS);
        expect(matchRouteCommand('PLAN')).toBe(Routes.HOME);
        expect(matchRouteCommand('do')).toBe(Routes.DO);
        expect(matchRouteCommand('buy milk')).toBeUndefined();
    });

    it('filters today tasks by start or due prefix', () => {
        const tasks: AppTask[] = [
            { id: '1', title: 'Due today', status: 'todo', due: '2026-11-19T14:00:00' },
            { id: '2', title: 'Started today', status: 'todo', start: '2026-11-19' },
            { id: '3', title: 'Tomorrow', status: 'todo', due: '2026-11-20' },
            { id: '4', title: 'Undated', status: 'todo' },
        ];

        expect(filterTodayTasks(tasks, '2026-11-19').map((t) => t.id)).toEqual(['1', '2']);
    });

    it('builds agenda rows without a create row when empty, and prepends create + filters when typed', () => {
        const events = [
            makeEvent('e1', 'Design sync', { kind: 'allDay', startDate: '2026-11-19', endDateExclusive: '2026-11-20' }),
            makeEvent('e2', 'Lunch', { kind: 'allDay', startDate: '2026-11-19', endDateExclusive: '2026-11-20' }),
        ];
        const tasks: AppTask[] = [
            { id: 't1', title: 'Review design doc', status: 'todo', due: '2026-11-19' },
            { id: 't2', title: 'Buy groceries', status: 'todo', start: '2026-11-19' },
        ];

        const emptyRows = buildPaletteRows('   ', events, tasks);
        expect(emptyRows.map((r) => r.key)).toEqual(['event:e1:0', 'event:e2:0', 'task:t1', 'task:t2']);

        const filteredRows = buildPaletteRows('design', events, tasks);
        expect(filteredRows.map((r) => r.key)).toEqual(['create', 'event:e1:0', 'task:t1']);
    });

    it('formats quick-capture preview parts and event/task details', () => {
        const preview = parseQuickCapture(
            'Ship release tomorrow at 9:30am #ops p2 ^45m +work',
            TODAY,
            SAMPLE_CALENDARS,
        );
        expect(formatPreviewParts(preview, TODAY)).toEqual([
            'Tomorrow 09:30',
            '#ops',
            'P3',
            '45 min',
            'Work',
        ]);

        const allDay = makeEvent('e1', 'Offsite', {
            kind: 'allDay',
            startDate: '2026-11-19',
            endDateExclusive: '2026-11-20',
        });
        expect(formatEventDetail(allDay)).toBe('All day');

        const timed = makeEvent('e2', 'Standup', {
            kind: 'timed',
            start: new Date(2026, 10, 19, 9, 30, 0).toISOString(),
            end: new Date(2026, 10, 19, 10, 0, 0).toISOString(),
            timezone: null,
        });
        expect(formatEventDetail(timed)).toBe('09:30–10:00');

        expect(formatTaskDetail({ id: 't1', title: 'A', status: 'todo', due: '2026-11-19' }, '2026-11-19')).toBe(
            'Due today',
        );
        expect(formatTaskDetail({ id: 't2', title: 'B', status: 'todo', start: '2026-11-19' }, '2026-11-19')).toBe(
            'Planned today',
        );
    });

    it('sets remoteCollectionId only for explicit +list queries and omits it for default captures', () => {
        const explicit = parseQuickCapture('Pay bill +work', TODAY, SAMPLE_CALENDARS, '/tasks/');
        const explicitTask = buildTaskFromQuickCapture(explicit, 'UTC');
        expect(explicitTask.remoteCollectionId).toBe('/work/');
        expect(explicitTask.timezone).toBeUndefined();

        const defaultCapture = parseQuickCapture('Call dentist tomorrow at 2pm', TODAY, SAMPLE_CALENDARS, '/work/');
        const defaultTask = buildTaskFromQuickCapture(defaultCapture, 'America/Chicago');
        expect(defaultCapture.list?.id).toBe('/work/');
        expect(defaultTask.remoteCollectionId).toBeUndefined();
        expect(defaultTask.timezone).toBe('America/Chicago');
        expect(defaultTask.due).toBe('2026-11-20T14:00:00');
    });
});
