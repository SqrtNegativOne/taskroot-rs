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
