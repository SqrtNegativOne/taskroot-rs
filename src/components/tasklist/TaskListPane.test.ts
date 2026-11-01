import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import type { AppCalendar, AppTaskColumnDef } from '../../lib/domain';
import TaskListPane from './TaskListPane.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

const calendars: AppCalendar[] = [
    { id: 'cal-events', summary: 'Event Calendar', writable: true, component: 'VEVENT' },
    { id: 'cal-tasks', summary: 'Task List', writable: true, component: 'VTODO' },
];

const schema: AppTaskColumnDef[] = [
    { id: 'remotecollectionid', label: 'Collection', filter_type: 'Text', sortable: false },
];

describe('TaskListPane calendar filter', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('get_active_calendars', calendars);
        tauri.stubCommandValue('get_task_schema', schema);
        tauri.stubCommandValue('query_tasks', []);
        tauri.stubCommandValue('get_past_due_task_ids', []);
        tauri.stubCommandValue('get_ui_state', undefined);
    });

    afterEach(() => {
        cleanup();
    });

    it('offers only task lists as calendar filter values', async () => {
        render(TaskListPane, { props: { onAddTask: vi.fn() } });

        await fireEvent.click(await screen.findByTitle('Filter'));
        await fireEvent.click(screen.getByRole('button', { name: /add filter/i }));
        await fireEvent.click(await screen.findByRole('button', { name: /select\.\.\./i }));

        expect(screen.getByRole('checkbox', { name: 'Task List' })).toBeTruthy();
        expect(screen.queryByRole('checkbox', { name: 'Event Calendar' })).toBeNull();
    });
});
