import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import { emptyFilterGroup, filterGroup, type AppCalendar } from '../domain';
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
        tauri.stubCommandValue('query_tasks', []);
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it("queries today's agenda while the input is empty", async () => {
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
            'query_tasks',
            { filters: filterGroup([{ column: 'status', operator: 'is not', value: ['done'] }]), sort: [], query: '' },
            undefined,
        );
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
        });

        vi.setSystemTime(new Date(2026, 10, 21, 0, 1, 0));
        window.dispatchEvent(new Event('focus'));

        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'query_event_instances',
                expect.objectContaining({ startDate: '2026-11-21', endDate: '2026-11-21' }),
                undefined,
            );
        });
    });
});
