import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import { makeTaskDateInstance } from '../../test/factories';
import { emptyFilterGroup, type AppCalendar, type AppTask } from '../domain';
import { store } from '../store.svelte';
import { ymd } from '../time';
import LauncherPalette from './LauncherPalette.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

const TASK_CALENDARS: AppCalendar[] = [
    { id: '/tasks/', summary: 'Tasks', writable: true, component: 'VTODO' },
    { id: '/events/', summary: 'Events', writable: true, component: 'VEVENT' },
];

function callFor(command: string) {
    return tauri.invoke.mock.calls.find(([name]) => name === command);
}

async function type(value: string): Promise<HTMLElement> {
    const input = screen.getByRole('textbox');
    await fireEvent.input(input, { target: { value } });
    return input;
}

describe('LauncherPalette', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('get_active_calendars', TASK_CALENDARS);
        tauri.stubCommandValue('query_event_instances', []);
        tauri.stubCommandValue('query_task_dates', []);
    });

    afterEach(() => {
        vi.useRealTimers();
        store.settings = null;
    });

    it("queries today's projected events and task dates while the input is empty", async () => {
        render(LauncherPalette);

        const today = ymd(new Date());
        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'query_event_instances',
                expect.objectContaining({ startDate: today, endDate: today, filters: emptyFilterGroup() }),
                undefined,
            );
        });
        expect(tauri.invoke).toHaveBeenCalledWith(
            'query_task_dates',
            { startDate: today, endDate: today },
            undefined,
        );
        expect(callFor('query_tasks')).toBeUndefined();
    });

    it('matches local-day timed tasks via projected TaskDateInstance markers instead of raw date prefixes', async () => {
        vi.useFakeTimers({ toFake: ['Date'] });
        vi.setSystemTime(new Date(2026, 10, 19, 10, 0, 0));

        const earlyToday = new Date(2026, 10, 19, 0, 30, 0);
        const lateToday = new Date(2026, 10, 19, 23, 30, 0);
        const lateYesterday = new Date(2026, 10, 18, 23, 30, 0);
        const earlyTomorrow = new Date(2026, 10, 20, 0, 30, 0);

        tauri.stubCommandValue('query_task_dates', [
            makeTaskDateInstance({
                taskId: 'early-today',
                kind: 'due',
                title: 'Early local task',
                status: 'todo',
                timing: { kind: 'timed', start: earlyToday.toISOString(), timezone: null },
            }),
            makeTaskDateInstance({
                taskId: 'late-today',
                kind: 'start',
                title: 'Late local task',
                status: 'next-up',
                timing: { kind: 'timed', start: lateToday.toISOString(), timezone: null },
            }),
            makeTaskDateInstance({
                taskId: 'done-today',
                kind: 'due',
                title: 'Done today task',
                status: 'done',
                timing: { kind: 'allDay', date: '2026-11-19' },
            }),
            makeTaskDateInstance({
                taskId: 'yesterday',
                kind: 'due',
                title: 'Yesterday task',
                status: 'todo',
                timing: { kind: 'timed', start: lateYesterday.toISOString(), timezone: null },
            }),
            makeTaskDateInstance({
                taskId: 'tomorrow',
                kind: 'due',
                title: 'Tomorrow task',
                status: 'todo',
                timing: { kind: 'timed', start: earlyTomorrow.toISOString(), timezone: null },
            }),
        ]);

        render(LauncherPalette);

        await screen.findByText('Early local task');
        expect(screen.getByText('Due today')).not.toBeNull();
        expect(screen.getByText('Late local task')).not.toBeNull();
        expect(screen.getByText('Planned today')).not.toBeNull();
        expect(screen.queryByText('Done today task')).toBeNull();
        expect(screen.queryByText('Yesterday task')).toBeNull();
        expect(screen.queryByText('Tomorrow task')).toBeNull();
    });

    it('parses typed text synchronously on the client and previews the captured task', async () => {
        vi.useFakeTimers({ toFake: ['Date'] });
        vi.setSystemTime(new Date(2026, 10, 19, 10, 0, 0));

        render(LauncherPalette);
        await waitFor(() => expect(callFor('get_active_calendars')).toBeDefined());

        await type('Buy milk tomorrow at 9:30am #errands p2 ^30m +tasks');

        expect(screen.getByText('Buy milk')).not.toBeNull();
        expect(screen.getByText('Tomorrow 09:30 · #errands · P3 · 30 min · Tasks')).not.toBeNull();
        expect(callFor('parse_quick_capture')).toBeUndefined();
    });

    it('creates the previewed task with its due, tags, priority, estimate, list and zone', async () => {
        vi.useFakeTimers({ toFake: ['Date'] });
        vi.setSystemTime(new Date(2026, 10, 19, 10, 0, 0));

        render(LauncherPalette);
        await waitFor(() => expect(callFor('get_active_calendars')).toBeDefined());

        const input = await type('Buy milk tomorrow at 9:30am #work #urgent p3 ^45m +tasks');
        await screen.findByText('Buy milk');
        await fireEvent.keyDown(input, { key: 'Enter' });

        await waitFor(() => expect(callFor('create_task')).toBeDefined());
        expect(callFor('create_task')?.[1]).toMatchObject({
            task: {
                title: 'Buy milk',
                status: 'todo',
                priority: 2,
                due: '2026-11-20T09:30:00',
                est: 45,
                remoteCollectionId: '/tasks/',
                timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
                tags: [{ name: 'work' }, { name: 'urgent' }],
            },
        });
        await waitFor(() => expect(callFor('hide_launcher')).toBeDefined());
    });

    it('previews the configured default_task_list and leaves remoteCollectionId unset when no +list sigil is given', async () => {
        tauri.stubCommandValue('get_active_calendars', [
            { id: '/tasks/', summary: 'Tasks', writable: true, component: 'VTODO' },
            { id: '/work/', summary: 'Work', writable: true, component: 'VTODO' },
        ]);
        store.settings = { default_task_list: '/work/' };

        render(LauncherPalette);
        await waitFor(() => expect(callFor('get_active_calendars')).toBeDefined());

        const input = await type('Buy milk');
        await screen.findByText('Work');
        await fireEvent.keyDown(input, { key: 'Enter' });

        await waitFor(() => expect(callFor('create_task')).toBeDefined());
        const created = (callFor('create_task')?.[1] as { task: AppTask } | undefined)?.task;
        expect(created?.title).toBe('Buy milk');
        expect(created?.remoteCollectionId).toBeUndefined();
    });

    it('does not create a task when the preview reports a list error', async () => {
        render(LauncherPalette);
        await waitFor(() => expect(callFor('get_active_calendars')).toBeDefined());

        const input = await type('Buy milk +missing');
        await screen.findByText('Unknown task list "missing"');
        await fireEvent.keyDown(input, { key: 'Enter' });

        expect(callFor('create_task')).toBeUndefined();
    });

    it('hides the launcher on Escape', async () => {
        render(LauncherPalette);

        await fireEvent.keyDown(screen.getByRole('textbox'), { key: 'Escape' });

        await waitFor(() => expect(callFor('hide_launcher')).toBeDefined());
    });

    it('refreshes the agenda date range after midnight when reopened without remounting', async () => {
        vi.useFakeTimers({ toFake: ['Date'] });
        vi.setSystemTime(new Date(2026, 10, 20, 23, 59, 0));

        render(LauncherPalette);
        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'query_event_instances',
                expect.objectContaining({ startDate: '2026-11-20', endDate: '2026-11-20' }),
                undefined,
            );
            expect(tauri.invoke).toHaveBeenCalledWith(
                'query_task_dates',
                { startDate: '2026-11-20', endDate: '2026-11-20' },
                undefined,
            );
        });

        vi.setSystemTime(new Date(2026, 10, 21, 0, 1, 0));
        window.dispatchEvent(new Event('focus'));

        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'query_event_instances',
                expect.objectContaining({ startDate: '2026-11-21', endDate: '2026-11-21' }),
                undefined,
            );
            expect(tauri.invoke).toHaveBeenCalledWith(
                'query_task_dates',
                { startDate: '2026-11-21', endDate: '2026-11-21' },
                undefined,
            );
        });
    });
});
