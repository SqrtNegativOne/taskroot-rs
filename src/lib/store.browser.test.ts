import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import { waitFor } from '@testing-library/svelte';
import type { EventCallback } from '@tauri-apps/api/event';
import { mockTauriInvoke } from '../test/tauriMock';
import { mountEffectRoot } from './effectRootHarness.svelte';
import { useTauriQuery } from './safeInvoke.svelte';

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

describe('useTauriQuery debounced stale guard', () => {
    it('drops an in-flight response that resolves while a newer debounced request is queued', async () => {
        const first = Promise.withResolvers<string>();
        const second = Promise.withResolvers<string>();
        const command = vi.fn((term: string) => (term === 'a' ? first.promise : second.promise));

        let query!: ReturnType<typeof useTauriQuery<string, [string]>>;
        const dispose = mountEffectRoot(() => {
            query = useTauriQuery(command, { debounceMs: 60 });
        });
        flushSync();

        void query.execute('a');
        await waitFor(() => { expect(command).toHaveBeenCalledWith('a'); });

        // Queue a newer debounced request while 'a' is still in flight.
        const secondDone = query.execute('ab');
        expect(query.isLoading).toBe(true);

        // Stale 'a' resolves well before 'ab' finishes debouncing.
        first.resolve('stale-a');
        await new Promise((resolve) => setTimeout(resolve, 15));

        expect(query.data).toBeUndefined();
        expect(query.isLoading).toBe(true);

        await waitFor(() => { expect(command).toHaveBeenCalledWith('ab'); });
        second.resolve('fresh-ab');
        await secondDone;

        expect(query.data).toBe('fresh-ab');
        expect(query.isLoading).toBe(false);
        dispose();
    });
});
