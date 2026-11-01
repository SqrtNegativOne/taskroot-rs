import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke, type InvokeArgs } from '../../test/tauriMock';
import { makeEvent } from '../../test/factories';
import InspectorPane from './InspectorPane.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

function invokedArgs(command: string): InvokeArgs | undefined {
    return tauri.invoke.mock.calls.find(([name]) => name === command)?.[1];
}

const master = makeEvent({
    id: 'master',
    title: 'Standup',
    startTime: '2026-10-19T11:00:00',
    endTime: '2026-10-19T11:30:00',
    rrule: 'RRULE:FREQ=WEEKLY',
    timezone: 'Asia/Kolkata',
});

async function openDeleteModal(onClose = vi.fn()) {
    render(InspectorPane, {
        props: {
            inspectorState: {
                type: 'event',
                id: 'master',
                occurrenceStart: '2026-10-26T11:00:00',
            },
            onClose,
        },
    });

    await fireEvent.click(await screen.findByRole('button', { name: /delete event/i }));
    return onClose;
}

describe('InspectorPane pending creation', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('query_tasks', []);
        tauri.stubCommandValue('query_events', []);
        tauri.stubCommandValue('get_active_calendars', []);
        tauri.stubCommandValue('get_ui_state', undefined);
    });

    afterEach(() => {
        cleanup();
    });

    it('opens with the draft event before the backend query returns it', async () => {
        const draft = makeEvent({ id: 'draft-1', title: 'New Event' });
        const { container } = render(InspectorPane, {
            props: {
                inspectorState: { type: 'event', id: 'draft-1', draft },
                onClose: vi.fn(),
            },
        });

        await screen.findByDisplayValue('New Event');
        expect(container.querySelector('.inspector-pane')?.classList.contains('is-open')).toBe(true);
    });
});

describe('InspectorPane recurring delete', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('query_tasks', []);
        tauri.stubCommandValue('query_events', [master]);
        tauri.stubCommandValue('get_active_calendars', []);
        tauri.stubCommandValue('get_ui_state', undefined);
    });

    afterEach(() => {
        cleanup();
    });

    it.each([
        { rrule: undefined, recurringEventId: undefined },
        { rrule: null, recurringEventId: null },
    ])('deletes a non-recurring event without a scope prompt (%j)', async (recurrence) => {
        tauri.stubCommandValue('query_events', [{ ...makeEvent({ id: 'single' }), ...recurrence }]);
        const onClose = vi.fn();
        render(InspectorPane, {
            props: {
                inspectorState: { type: 'event', id: 'single', occurrenceStart: '2026-09-07T09:00:00' },
                onClose,
            },
        });

        await fireEvent.click(await screen.findByRole('button', { name: /delete event/i }));

        expect(screen.queryByRole('dialog')).toBeNull();
        await waitFor(() => {
            expect(invokedArgs('delete_event')).toEqual({ id: 'single', scope: 'all', occurrenceStart: undefined });
            expect(onClose).toHaveBeenCalledOnce();
        });
    });

    it('prompts for a recurrence exception even when it has no rule of its own', async () => {
        tauri.stubCommandValue('query_events', [{ ...master, rrule: null, recurringEventId: 'series' }]);
        const onClose = await openDeleteModal();

        expect(screen.getByRole('dialog')).toBeTruthy();
        expect(invokedArgs('delete_event')).toBeUndefined();
        expect(onClose).not.toHaveBeenCalled();

        await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
        expect(invokedArgs('delete_event')).toBeUndefined();
        expect(onClose).not.toHaveBeenCalled();
    });

    it('scopes the delete to the clicked occurrence, not the first one of the series', async () => {
        await openDeleteModal();

        await fireEvent.click(screen.getByRole('button', { name: 'OK' }));

        await waitFor(() => {
            expect(invokedArgs('delete_event')).toEqual({
                id: 'master',
                scope: 'this',
                occurrenceStart: '2026-10-26T11:00:00',
            });
        });
    });

    it('passes the selected scope through to the backend', async () => {
        await openDeleteModal();

        await fireEvent.click(screen.getByLabelText('This and following events'));
        await fireEvent.click(screen.getByRole('button', { name: 'OK' }));

        await waitFor(() => {
            expect(invokedArgs('delete_event')).toEqual({
                id: 'master',
                scope: 'following',
                occurrenceStart: '2026-10-26T11:00:00',
            });
        });
    });
});
