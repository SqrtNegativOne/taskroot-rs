import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { toast } from 'svelte-sonner';
import { mockTauriInvoke } from '../../test/tauriMock';
import { store } from '../../lib/store.svelte';
import PlanScreen from './PlanScreen.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

vi.mock('svelte-sonner', () => ({
    toast: {
        error: vi.fn(),
        success: vi.fn(),
        info: vi.fn(),
        custom: vi.fn(),
    },
    Toaster: vi.fn(),
}));

const tauri = mockTauriInvoke();

const task = {
    id: 'task-1',
    title: 'Buy milk',
    status: 'todo' as const,
};

describe('PlanScreen task list', () => {
    beforeEach(() => {
        tauri.reset();
        vi.clearAllMocks();
        store.loaded = true;
        store.error = null;
        tauri.stubCommandValue('query_tasks', [task]);
        tauri.stubCommandValue('get_past_due_task_ids', []);
        tauri.stubCommandValue('get_task_schema', []);
        tauri.stubCommandValue('get_active_calendars', []);
        tauri.stubCommandValue('query_event_instances', []);
        tauri.stubCommandValue('get_ui_state', undefined);
    });

    afterEach(() => {
        cleanup();
        store.loaded = false;
    });

    it('completes the task when its circle is clicked', async () => {
        render(PlanScreen);

        await screen.findByText('Buy milk');
        const row = screen.getByText('Buy milk').closest('.task-row');
        const circle = row?.querySelector('.task-circle');
        expect(circle).not.toBeNull();

        await fireEvent.click(circle!);

        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith('update_task', {
                task: { ...task, status: 'done' },
            });
        });
    });

    it('toasts a failed update instead of replacing the screen with a fatal error', async () => {
        tauri.stubCommand('update_task', () => Promise.reject({ code: 'remote', message: 'server down' }));
        render(PlanScreen);

        await screen.findByText('Buy milk');
        const row = screen.getByText('Buy milk').closest('.task-row');
        const circle = row?.querySelector('.task-circle');
        expect(circle).not.toBeNull();

        await fireEvent.click(circle!);

        await waitFor(() => {
            expect(toast.custom).toHaveBeenCalledTimes(1);
        });
        expect(toast.custom).toHaveBeenCalledWith(
            expect.anything(),
            expect.objectContaining({
                componentProps: { message: 'Failed to update task: server down' },
                duration: Infinity,
            }),
        );
        expect(store.error).toBeNull();
        expect(screen.getByText('Buy milk')).toBeTruthy();
        expect(screen.queryByText(/Error loading data from backend/)).toBeNull();
    });
});
