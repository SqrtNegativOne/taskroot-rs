import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import { store } from '../../lib/store.svelte';
import PlanScreen from './PlanScreen.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

// `@tauri-apps/api/event` is externalized under Vitest, so it reads these
// globals directly instead of going through the mocked `invoke`.
vi.hoisted(() => {
    let nextCallbackId = 0;
    const globals = window as Window & {
        __TAURI_INTERNALS__?: {
            transformCallback: (callback: unknown, once?: boolean) => number;
            invoke: (command: string, args?: unknown) => Promise<unknown>;
        };
        __TAURI_EVENT_PLUGIN_INTERNALS__?: {
            unregisterListener: (event: string, eventId: number) => void;
        };
    };
    globals.__TAURI_INTERNALS__ = {
        transformCallback: () => nextCallbackId++,
        invoke: () => Promise.resolve(undefined),
    };
    globals.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
        unregisterListener: () => { /* noop */ },
    };
});

const tauri = mockTauriInvoke();

const task = {
    id: 'task-1',
    title: 'Buy milk',
    status: 'todo' as const,
};

describe('PlanScreen task list', () => {
    beforeEach(() => {
        tauri.reset();
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
});
