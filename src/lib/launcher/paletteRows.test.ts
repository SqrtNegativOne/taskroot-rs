import { describe, expect, it } from 'vitest';
import type { AppCalendar, EventInstance, TaskDateInstance } from '../domain';
import { makeInstance, makeTaskDateInstance } from '../../test/factories';
import { Routes } from '../routes';
import { parseQuickCapture } from './quickCapture';
import {
    buildPaletteRows,
    buildTaskFromQuickCapture,
    formatEventDetail,
    formatPreviewParts,
    formatTaskDetail,
    matchRouteCommand,
    projectTodayTasks,
    type TodayTask,
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

    it('projects timed and all-day task markers onto the local calendar day, excluding done tasks and neighboring days', () => {
        const earlyToday = new Date(2026, 10, 19, 0, 30, 0);
        const lateToday = new Date(2026, 10, 19, 23, 30, 0);
        const lateYesterday = new Date(2026, 10, 18, 23, 30, 0);
        const earlyTomorrow = new Date(2026, 10, 20, 0, 30, 0);

        const markers: TaskDateInstance[] = [
            makeTaskDateInstance({
                taskId: 'early-today',
                kind: 'due',
                title: 'Early local due',
                status: 'todo',
                timing: { kind: 'timed', start: earlyToday.toISOString(), timezone: null },
            }),
            makeTaskDateInstance({
                taskId: 'late-today',
                kind: 'start',
                title: 'Late local start',
                status: 'next-up',
                timing: { kind: 'timed', start: lateToday.toISOString(), timezone: null },
            }),
            makeTaskDateInstance({
                taskId: 'all-day',
                kind: 'start',
                title: 'All-day start',
                status: 'todo',
                timing: { kind: 'allDay', date: '2026-11-19' },
            }),
            makeTaskDateInstance({
                taskId: 'done-today',
                kind: 'due',
                title: 'Already done',
                status: 'done',
                timing: { kind: 'allDay', date: '2026-11-19' },
            }),
            makeTaskDateInstance({
                taskId: 'yesterday',
                kind: 'due',
                title: 'Late yesterday',
                status: 'todo',
                timing: { kind: 'timed', start: lateYesterday.toISOString(), timezone: null },
            }),
            makeTaskDateInstance({
                taskId: 'tomorrow',
                kind: 'due',
                title: 'Early tomorrow',
                status: 'todo',
                timing: { kind: 'timed', start: earlyTomorrow.toISOString(), timezone: null },
            }),
            makeTaskDateInstance({
                taskId: 'both-markers',
                kind: 'start',
                title: 'Both start and due',
                status: 'todo',
                timing: { kind: 'allDay', date: '2026-11-19' },
            }),
            makeTaskDateInstance({
                taskId: 'both-markers',
                kind: 'due',
                title: 'Both start and due',
                status: 'todo',
                timing: { kind: 'allDay', date: '2026-11-19' },
            }),
        ];

        expect(projectTodayTasks(markers, TODAY)).toEqual([
            { taskId: 'early-today', title: 'Early local due', kind: 'due' },
            { taskId: 'late-today', title: 'Late local start', kind: 'start' },
            { taskId: 'all-day', title: 'All-day start', kind: 'start' },
            { taskId: 'both-markers', title: 'Both start and due', kind: 'due' },
        ]);
    });

    it('builds agenda rows without a create row when empty, and prepends create + filters when typed', () => {
        const events = [
            makeEvent('e1', 'Design sync', { kind: 'allDay', startDate: '2026-11-19', endDateExclusive: '2026-11-20' }),
            makeEvent('e2', 'Lunch', { kind: 'allDay', startDate: '2026-11-19', endDateExclusive: '2026-11-20' }),
        ];
        const tasks: TodayTask[] = [
            { taskId: 't1', title: 'Review design doc', kind: 'due' },
            { taskId: 't2', title: 'Buy groceries', kind: 'start' },
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

        expect(formatTaskDetail({ taskId: 't1', title: 'A', kind: 'due' })).toBe('Due today');
        expect(formatTaskDetail({ taskId: 't2', title: 'B', kind: 'start' })).toBe('Planned today');
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
