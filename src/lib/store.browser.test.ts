import { beforeEach, describe, expect, it, vi } from 'vitest';
import { waitFor } from '@testing-library/svelte';
import type { EventCallback } from '@tauri-apps/api/event';
import { mockTauriInvoke } from '../test/tauriMock';

const eventMock = vi.hoisted(() => ({
    listeners: new Map<string, Set<EventCallback<unknown>>>(),
    emit: vi.fn<(...args: unknown[]) => Promise<void>>(() => Promise.resolve()),
}));

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

vi.mock('@tauri-apps/api/event', () => ({
    listen: (name: string, handler: EventCallback<unknown>) => {
        const set = eventMock.listeners.get(name) ?? new Set<EventCallback<unknown>>();
        eventMock.listeners.set(name, set);
        set.add(handler);
        return Promise.resolve(() => { set.delete(handler); });
    },
    once: vi.fn(() => Promise.resolve(() => { /* noop */ })),
    emit: (...args: unknown[]) => eventMock.emit(...args),
    emitTo: vi.fn(() => Promise.resolve()),
}));

import { store } from './store.svelte';

const tauri = mockTauriInvoke();

function publish(name: string): void {
    for (const handler of eventMock.listeners.get(name) ?? []) {
        handler({ event: name, id: 1, payload: null });
    }
}

describe('AppStore IPC & event contract', () => {
    beforeEach(() => {
        tauri.reset();
        eventMock.emit.mockClear();
        store.settings = null;
    });

    it('sends field patches without emitting store-updated from the frontend', async () => {
        await store.updateTask('task-1', [{ field: 'title', value: 'Patched' }]);
        await store.updateEvent('event-1', [{ field: 'color', value: '#123456' }]);
        await store.toggleTask('task-1');

        expect(tauri.invoke).toHaveBeenCalledWith(
            'update_task',
            { id: 'task-1', patches: [{ field: 'title', value: 'Patched' }] },
            undefined,
        );
        expect(tauri.invoke).toHaveBeenCalledWith(
            'update_event',
            { id: 'event-1', patches: [{ field: 'color', value: '#123456' }] },
            undefined,
        );
        expect(eventMock.emit).not.toHaveBeenCalled();
    });

    it('does not re-fetch settings on store-updated', async () => {
        tauri.stubCommandValue('get_settings', { colour_scheme: 'ember' });

        publish('store-updated');
        await Promise.resolve();
        await Promise.resolve();

        expect(tauri.invoke).not.toHaveBeenCalledWith('get_settings', undefined, undefined);
        expect(store.settings).toBeNull();
    });

    it('refreshes settings when settings-updated is received', async () => {
        tauri.stubCommandValue('get_settings', { colour_scheme: 'ember' });

        publish('settings-updated');

        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith('get_settings', undefined, undefined);
            expect(store.settings).toEqual({ colour_scheme: 'ember' });
        });
    });
});
