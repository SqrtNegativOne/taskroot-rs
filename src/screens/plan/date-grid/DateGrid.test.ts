import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../../test/tauriMock';
import { makeInstance, makeTaskDateInstance } from '../../../test/factories';
import { addDays, startOfWeek, ymd } from '../../../lib/time';
import { persistedKeys } from '../../../lib/persisted.svelte';
import { filterGroup } from '../../../lib/domain';
import type { AppCalendar } from '../../../lib/domain';
import DateGrid from './DateGrid.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();
const calendars: AppCalendar[] = [
    { id: 'events', summary: 'Event Calendar', writable: true, component: 'VEVENT' },
    { id: 'tasks', summary: 'Task List', writable: true, component: 'VTODO' },
];

async function openFilters() {
    await fireEvent.click(screen.getByTitle('Filter'));
    return {
        recurring: screen.getByRole('checkbox', { name: 'Hide daily or weekly recurring events' }),
        timed: screen.getByRole('checkbox', { name: 'Hide timed events' }),
        done: screen.getByRole('checkbox', { name: 'Hide completed tasks' }),
    };
}

describe('DateGrid visibility filters', () => {
    beforeEach(() => {
        tauri.reset();
        const start = startOfWeek(new Date());
        const startDate = ymd(start);
        const endDateExclusive = ymd(addDays(start, 1));
        const timedStart = new Date(start);
        timedStart.setHours(9);
        const timedEnd = new Date(start);
        timedEnd.setHours(10);
        const timing = { start: timedStart.toISOString(), end: timedEnd.toISOString() };
        tauri.stubCommandValue('get_active_calendars', calendars);
        tauri.stubCommandValue('get_ui_state', undefined);
        tauri.stubCommandValue('query_event_instances', [
            makeInstance({ id: 'all-day', title: 'One-off all day', timing: { kind: 'allDay', startDate, endDateExclusive } }),
            makeInstance({ id: 'timed', title: 'One-off timed', timing }),
            makeInstance({ id: 'repeat', title: 'Recurring timed', recurring: true, rrule: 'RRULE:FREQ=DAILY', timing }),
            makeInstance({ id: 'exception', title: 'Moved exception', recurring: true, rrule: 'RRULE:FREQ=WEEKLY', timing }),
            makeInstance({ id: 'span', title: 'Recurring span', recurring: true, rrule: 'RRULE:FREQ=WEEKLY', timing: { kind: 'allDay', startDate, endDateExclusive: ymd(addDays(start, 3)) } }),
            makeInstance({ id: 'yearly', title: 'Yearly event', recurring: true, rrule: 'RRULE:FREQ=YEARLY', timing }),
        ]);
    });

    it('shows all event kinds by default and offers only event calendars', async () => {
        render(DateGrid);
        await screen.findByText('One-off all day');
        expect(screen.getByText('One-off timed')).toBeTruthy();
        expect(screen.getByText('Recurring timed')).toBeTruthy();
        expect(screen.getByRole('button', { name: 'Recurring span' })).toBeTruthy();
        expect(screen.getByText('Yearly event')).toBeTruthy();
        const { recurring, timed, done } = await openFilters();
        expect(recurring).toHaveProperty('checked', false);
        expect(timed).toHaveProperty('checked', false);
        expect(done).toHaveProperty('checked', false);
        await fireEvent.click(screen.getByRole('button', { name: /add filter/i }));
        await fireEvent.click(screen.getByRole('button', { name: /select\.\.\./i }));
        expect(screen.getByRole('checkbox', { name: 'Event Calendar' })).toBeTruthy();
        expect(screen.queryByRole('checkbox', { name: 'Task List' })).toBeNull();
    });

    it('hides daily and weekly recurring occurrences, exceptions and spanning bars, but keeps less frequent series', async () => {
        render(DateGrid);
        await screen.findByText('Recurring timed');
        const { recurring } = await openFilters();
        await fireEvent.click(recurring);
        expect(screen.queryByText('Recurring timed')).toBeNull();
        expect(screen.queryByText('Moved exception')).toBeNull();
        expect(screen.queryByRole('button', { name: 'Recurring span' })).toBeNull();
        expect(screen.getByText('Yearly event')).toBeTruthy();
        expect(screen.getByText('One-off timed')).toBeTruthy();
        expect(screen.getByText('One-off all day')).toBeTruthy();
        await fireEvent.click(recurring);
        expect(screen.getByText('Moved exception')).toBeTruthy();
        expect(screen.getByRole('button', { name: 'Recurring span' })).toBeTruthy();
    });

    it('hides timed events independently and combines both choices', async () => {
        render(DateGrid);
        await screen.findByText('One-off timed');
        const { recurring, timed } = await openFilters();
        await fireEvent.click(timed);
        expect(screen.queryByText('One-off timed')).toBeNull();
        expect(screen.queryByText('Recurring timed')).toBeNull();
        expect(screen.getByRole('button', { name: 'Recurring span' })).toBeTruthy();
        await fireEvent.click(recurring);
        expect(screen.queryByRole('button', { name: 'Recurring span' })).toBeNull();
        expect(screen.getByText('One-off all day')).toBeTruthy();
        expect(screen.getByTitle('Filter').textContent?.trim()).toBe('2');
    });

    it('persists choices and restores them on remount without overwriting calendar filters', async () => {
        const stored = new Map<string, unknown>([
            [persistedKeys.dateGridFilters, filterGroup([{ column: 'remotecollectionid', operator: 'is', value: ['events'] }])],
        ]);
        tauri.stubCommand('get_ui_state', args => stored.get(String(args?.key)));
        tauri.stubCommand('set_ui_state', args => { stored.set(String(args?.key), args?.value); });
        const mounted = render(DateGrid);
        await screen.findByText('One-off timed');
        const { timed } = await openFilters();
        await fireEvent.click(timed);
        mounted.unmount();
        await waitFor(() => {
            expect(stored.get(persistedKeys.dateGridVisibility)).toEqual({ hideFrequentRecurring: false, hideTimed: true, hideDoneTasks: false });
        });
        render(DateGrid);
        await screen.findByText('One-off all day');
        expect(screen.queryByText('One-off timed')).toBeNull();
        const restored = await openFilters();
        expect(restored.timed).toHaveProperty('checked', true);
        expect(screen.getByTitle('Filter').textContent?.trim()).toBe('2');
        expect(screen.getByRole('button', { name: /event calendar/i })).toBeTruthy();
    });

    it('ignores malformed stored visibility instead of accidentally hiding events', async () => {
        tauri.stubCommand('get_ui_state', args => args?.key === persistedKeys.dateGridVisibility
            ? { hideFrequentRecurring: 'false', hideTimed: true }
            : undefined);
        render(DateGrid);
        await screen.findByText('One-off timed');
        const { recurring, timed } = await openFilters();
        expect(recurring).toHaveProperty('checked', false);
        expect(timed).toHaveProperty('checked', false);
    });
});

describe('DateGrid done-task hiding', () => {
    beforeEach(() => {
        tauri.reset();
        const start = startOfWeek(new Date());
        const startDate = ymd(start);
        tauri.stubCommandValue('get_active_calendars', calendars);
        tauri.stubCommandValue('get_ui_state', undefined);
        tauri.stubCommandValue('query_event_instances', [
            makeInstance({ id: 'planned-done', title: 'Planned done', taskId: 't1', taskStatus: 'done', timing: { kind: 'allDay', startDate, endDateExclusive: ymd(addDays(start, 1)) } }),
            makeInstance({ id: 'planned-todo', title: 'Planned todo', taskId: 't2', taskStatus: 'todo', timing: { kind: 'allDay', startDate, endDateExclusive: ymd(addDays(start, 1)) } }),
            makeInstance({ id: 'plain', title: 'Plain event', timing: { kind: 'allDay', startDate, endDateExclusive: ymd(addDays(start, 1)) } }),
        ]);
        tauri.stubCommandValue('query_task_dates', [
            makeTaskDateInstance({ taskId: 't1', kind: 'due', title: 'Done marker', status: 'done', timing: { kind: 'allDay', date: startDate } }),
            makeTaskDateInstance({ taskId: 't2', kind: 'due', title: 'Open marker', status: 'todo', timing: { kind: 'allDay', date: ymd(addDays(start, 1)) } }),
        ]);
    });

    it('hides done planned events and done task markers only after opting in', async () => {
        render(DateGrid);
        expect(await screen.findByText('Planned done')).toBeTruthy();
        expect(await screen.findByText('Done marker')).toBeTruthy();

        const { done } = await openFilters();
        await fireEvent.click(done);

        expect(screen.queryByText('Planned done')).toBeNull();
        expect(screen.queryByText('Done marker')).toBeNull();
        expect(screen.getByText('Planned todo')).toBeTruthy();
        expect(screen.getByText('Plain event')).toBeTruthy();
        expect(screen.getByText('Open marker')).toBeTruthy();
        expect(screen.getByTitle('Filter').textContent?.trim()).toBe('1');
    });
});

describe('DateGrid event titles', () => {
    beforeEach(() => {
        tauri.reset();
        const start = startOfWeek(new Date());
        tauri.stubCommandValue('get_active_calendars', calendars);
        tauri.stubCommandValue('get_ui_state', undefined);
        tauri.stubCommandValue('query_event_instances', [
            makeInstance({
                id: 'unnamed',
                title: '',
                timing: { kind: 'allDay', startDate: ymd(start), endDateExclusive: ymd(addDays(start, 1)) },
            }),
        ]);
    });

    it('labels an unnamed event instead of rendering an empty box', async () => {
        render(DateGrid);
        expect(await screen.findByText('Untitled')).toBeTruthy();
    });
});

describe('DateGrid task dates', () => {
    beforeEach(() => {
        tauri.reset();
        const start = startOfWeek(new Date());
        const startDate = ymd(start);
        tauri.stubCommandValue('get_active_calendars', calendars);
        tauri.stubCommandValue('get_ui_state', undefined);
        tauri.stubCommandValue('query_event_instances', []);
        tauri.stubCommandValue('query_task_dates', [
            makeTaskDateInstance({
                taskId: 'rent',
                kind: 'start',
                title: 'Pay rent',
                timing: { kind: 'allDay', date: startDate },
            }),
            makeTaskDateInstance({
                taskId: 'rent',
                kind: 'due',
                title: 'Pay rent',
                timing: { kind: 'allDay', date: startDate },
            }),
            makeTaskDateInstance({
                taskId: 'call',
                kind: 'due',
                title: 'Call mom',
                timing: { kind: 'allDay', date: ymd(addDays(start, 1)) },
            }),
        ]);
    });

    it('renders a start and due marker even when they land on the same day', async () => {
        const onTaskClick = vi.fn();
        render(DateGrid, { onTaskClick });

        expect(await screen.findAllByText('Pay rent')).toHaveLength(2);

        expect(screen.getByText('Call mom')).toBeTruthy();
    });

    it('opens the task when a marker is clicked', async () => {
        const onTaskClick = vi.fn();
        render(DateGrid, { onTaskClick });

        await fireEvent.click((await screen.findAllByText('Pay rent'))[0]);

        expect(onTaskClick).toHaveBeenCalledTimes(1);
        expect(onTaskClick.mock.calls[0][0]).toMatchObject({ taskId: 'rent', kind: 'start' });
    });
});
