import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import type { EventCallback } from '@tauri-apps/api/event';
import { mockTauriInvoke } from '../test/tauriMock';
import { makeEvent } from '../test/factories';
import type { AppCalendar, AppTask } from './domain';
import { mountEffectRoot } from './effectRootHarness.svelte';
import { useActiveCalendars, useTags } from './sharedQueries.svelte';
import EventFilterButton from '../components/EventFilterButton.svelte';
import InspectorPane from '../components/inspector-pane/InspectorPane.svelte';

const eventMock = vi.hoisted(() => ({
    listeners: new Map<string, Set<EventCallback<unknown>>>(),
    deferredListens: [] as Array<() => void>,
    deferNextListen: false,
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
        const unlisten = () => { set.delete(handler); };
        if (eventMock.deferNextListen) {
            eventMock.deferNextListen = false;
            return new Promise<() => void>((resolve) => {
                eventMock.deferredListens.push(() => { resolve(unlisten); });
            });
        }
        return Promise.resolve(unlisten);
    },
    once: vi.fn(() => Promise.resolve(() => { /* noop */ })),
    emit: vi.fn(() => Promise.resolve()),
    emitTo: vi.fn(() => Promise.resolve()),
}));

const tauri = mockTauriInvoke();

function publishStoreUpdated(): void {
    for (const handler of eventMock.listeners.get('store-updated') ?? []) {
        handler({ event: 'store-updated', id: 1, payload: null });
    }
}

function invokeCount(command: string): number {
    return tauri.invoke.mock.calls.filter(([name]) => name === command).length;
}

describe('sharedQueries window-level singletons', () => {
    beforeEach(() => {
        tauri.reset();
        eventMock.listeners.clear();
        eventMock.deferredListens.length = 0;
        eventMock.deferNextListen = false;
        tauri.stubCommandValue('get_active_calendars', []);
        tauri.stubCommandValue('list_tags', []);
        tauri.stubCommandValue('query_tasks', []);
        tauri.stubCommandValue('query_events', []);
        tauri.stubCommandValue('get_ui_state', undefined);
    });

    afterEach(() => {
        cleanup();
    });

    it('shares one get_active_calendars subscription across concurrent consumers and tears down when the last unsubscribes', async () => {
        const initial: AppCalendar[] = [
            { id: 'cal-1', summary: 'Personal', writable: true, component: 'VEVENT' },
        ];
        const updated: AppCalendar[] = [
            ...initial,
            { id: 'cal-2', summary: 'Work', writable: true, component: 'VEVENT' },
        ];
        tauri.stubCommandValue('get_active_calendars', initial);

        let q1!: ReturnType<typeof useActiveCalendars>;
        let q2!: ReturnType<typeof useActiveCalendars>;
        let q3!: ReturnType<typeof useActiveCalendars>;

        const dispose1 = mountEffectRoot(() => { q1 = useActiveCalendars(); });
        const dispose2 = mountEffectRoot(() => { q2 = useActiveCalendars(); });
        const dispose3 = mountEffectRoot(() => { q3 = useActiveCalendars(); });
        flushSync();

        try {
            await waitFor(() => {
                expect(q1.data).toEqual(initial);
                expect(q2.data).toEqual(initial);
                expect(q3.data).toEqual(initial);
            });
            expect(invokeCount('get_active_calendars')).toBe(1);
            expect(eventMock.listeners.get('store-updated')?.size ?? 0).toBe(1);

            tauri.stubCommandValue('get_active_calendars', updated);
            publishStoreUpdated();

            await waitFor(() => {
                expect(q1.data).toEqual(updated);
                expect(q2.data).toEqual(updated);
                expect(q3.data).toEqual(updated);
            });
            expect(invokeCount('get_active_calendars')).toBe(2);

            dispose1();
            dispose2();
            expect(eventMock.listeners.get('store-updated')?.size ?? 0).toBe(1);
        } finally {
            dispose3();
        }

        expect(eventMock.listeners.get('store-updated')?.size ?? 0).toBe(0);
        publishStoreUpdated();
        await Promise.resolve();
        expect(invokeCount('get_active_calendars')).toBe(2);
    });

    it('shares one list_tags subscription across multiple EventFilterButtons and refetches once per store-updated', async () => {
        const calendars: AppCalendar[] = [
            { id: 'events', summary: 'Events', writable: true, component: 'VEVENT' },
        ];
        tauri.stubCommandValue('list_tags', ['work', 'home']);

        const first = render(EventFilterButton, { props: { calendars } });
        const second = render(EventFilterButton, { props: { calendars } });
        const third = render(EventFilterButton, { props: { calendars } });

        let tagReader!: ReturnType<typeof useTags>;
        const disposeReader = mountEffectRoot(() => { tagReader = useTags(); });
        flushSync();

        try {
            await waitFor(() => {
                expect(tagReader.data).toEqual(['work', 'home']);
            });
            expect(invokeCount('list_tags')).toBe(1);

            tauri.stubCommandValue('list_tags', ['work', 'home', 'urgent']);
            publishStoreUpdated();

            await waitFor(() => {
                expect(tagReader.data).toEqual(['work', 'home', 'urgent']);
            });
            expect(invokeCount('list_tags')).toBe(2);
        } finally {
            disposeReader();
            first.unmount();
            second.unmount();
            third.unmount();
        }

        expect(eventMock.listeners.get('store-updated')?.size ?? 0).toBe(0);
    });

    it('cleans up a stale listen() promise that resolves after unmount and remount', async () => {
        const calendars: AppCalendar[] = [
            { id: 'cal-1', summary: 'Personal', writable: true, component: 'VEVENT' },
        ];
        tauri.stubCommandValue('get_active_calendars', calendars);

        eventMock.deferNextListen = true;
        const disposeFirst = mountEffectRoot(() => { useActiveCalendars(); });
        flushSync();
        expect(eventMock.deferredListens).toHaveLength(1);

        disposeFirst();

        let second!: ReturnType<typeof useActiveCalendars>;
        const disposeSecond = mountEffectRoot(() => { second = useActiveCalendars(); });
        flushSync();

        try {
            // Resolve the stale first listen() after the second subscriber is already active.
            eventMock.deferredListens.shift()?.();
            await waitFor(() => {
                expect(second.data).toEqual(calendars);
                expect(eventMock.listeners.get('store-updated')?.size ?? 0).toBe(1);
            });

            const callsBefore = invokeCount('get_active_calendars');
            publishStoreUpdated();
            await waitFor(() => {
                expect(invokeCount('get_active_calendars')).toBe(callsBefore + 1);
            });
        } finally {
            disposeSecond();
        }

        expect(eventMock.listeners.get('store-updated')?.size ?? 0).toBe(0);
    });
});

describe('lazy InspectorPaneState queries', () => {
    beforeEach(() => {
        tauri.reset();
        eventMock.listeners.clear();
        eventMock.deferredListens.length = 0;
        eventMock.deferNextListen = false;
        tauri.stubCommandValue('get_active_calendars', []);
        tauri.stubCommandValue('list_tags', ['focus']);
        tauri.stubCommandValue('query_tasks', []);
        tauri.stubCommandValue('query_events', []);
        tauri.stubCommandValue('get_ui_state', undefined);
    });

    afterEach(() => {
        cleanup();
    });

    it('runs zero task, event, tag, or calendar queries while closed and only queries needed data when opened', async () => {
        const task: AppTask = { id: 'task-1', title: 'Write tests' };
        const event = makeEvent({ id: 'ev-1', title: 'Design review' });
        tauri.stubCommandValue('query_tasks', [task]);
        tauri.stubCommandValue('query_events', [event]);

        const onClose = vi.fn();
        const { rerender } = render(InspectorPane, {
            props: {
                inspectorState: undefined,
                onClose,
            },
        });

        publishStoreUpdated();
        await Promise.resolve();
        await Promise.resolve();

        expect(invokeCount('query_tasks')).toBe(0);
        expect(invokeCount('query_events')).toBe(0);
        expect(invokeCount('list_tags')).toBe(0);
        expect(invokeCount('get_active_calendars')).toBe(0);

        await rerender({
            inspectorState: { type: 'task', id: task.id },
            onClose,
        });
        await screen.findByDisplayValue('Write tests');

        expect(invokeCount('query_tasks')).toBe(1);
        expect(invokeCount('query_events')).toBe(0);
        expect(invokeCount('list_tags')).toBe(1);
        expect(invokeCount('get_active_calendars')).toBe(1);

        await rerender({
            inspectorState: { type: 'event', id: event.id },
            onClose,
        });
        await screen.findByDisplayValue('Design review');

        expect(invokeCount('query_events')).toBe(1);

        await rerender({
            inspectorState: undefined,
            onClose,
        });

        const tasksBefore = invokeCount('query_tasks');
        const eventsBefore = invokeCount('query_events');
        const tagsBefore = invokeCount('list_tags');
        const calendarsBefore = invokeCount('get_active_calendars');

        publishStoreUpdated();
        await Promise.resolve();
        await Promise.resolve();

        expect(invokeCount('query_tasks')).toBe(tasksBefore);
        expect(invokeCount('query_events')).toBe(eventsBefore);
        expect(invokeCount('list_tags')).toBe(tagsBefore);
        expect(invokeCount('get_active_calendars')).toBe(calendarsBefore);
    });
});
