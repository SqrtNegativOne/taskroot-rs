import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import { makeEvent } from '../../test/factories';
import type { AppCalendar, AppEvent } from '../../lib/domain';
import InspectorEventFields from './InspectorEventFields.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

const calendars: AppCalendar[] = [
    { id: 'cal-events', summary: 'Event Calendar', writable: true, component: 'VEVENT', color: '#123456' },
    { id: 'cal-other', summary: 'Other Calendar', writable: true, component: 'VEVENT', color: '#654321' },
];

function renderFields(event: AppEvent) {
    const updateEvent = vi.fn();
    render(InspectorEventFields, {
        props: { event, tasks: [], updateEvent },
    });
    return updateEvent;
}

function lastUpdate(updateEvent: ReturnType<typeof vi.fn>, event: AppEvent): AppEvent {
    const updater = updateEvent.mock.calls.at(-1)?.[1] as ((current: AppEvent) => AppEvent) | undefined;
    expect(updater).toBeDefined();
    return updater!(event);
}

describe('event colour field', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('get_active_calendars', calendars);
    });

    afterEach(() => {
        cleanup();
    });

    it('shows the calendar colour when the event has no override', async () => {
        renderFields(makeEvent({ remoteCollectionId: 'cal-events' }));

        const input = (await screen.findByLabelText('Event colour')) as HTMLInputElement;
        await waitFor(() => expect(input.value).toBe('#123456'));
        expect(screen.getByText('From calendar').textContent).toBe('From calendar');
    });

    it('writes the picked colour to the event as an override', async () => {
        const event = makeEvent({ remoteCollectionId: 'cal-events' });
        const updateEvent = renderFields(event);
        const input = await screen.findByLabelText('Event colour');

        await fireEvent.change(input, { target: { value: '#ff0000' } });

        expect(lastUpdate(updateEvent, event).color).toBe('#ff0000');
    });

    it('clears the override so the event follows its calendar again', async () => {
        const event = makeEvent({ remoteCollectionId: 'cal-events', color: '#ff0000' });
        const updateEvent = renderFields(event);

        await fireEvent.click(
            await screen.findByRole('button', { name: 'Clear event colour' }),
        );

        expect(lastUpdate(updateEvent, event).color).toBeUndefined();
    });

    it('keeps an explicit colour when the event moves to another calendar', async () => {
        const event = makeEvent({ remoteCollectionId: 'cal-events', color: '#ff0000' });
        const updateEvent = renderFields(event);

        await fireEvent.click(await screen.findByRole('button', { name: 'Calendar' }));
        await fireEvent.click(await screen.findByRole('button', { name: 'Other Calendar' }));

        const updated = lastUpdate(updateEvent, event);
        expect(updated.remoteCollectionId).toBe('cal-other');
        expect(updated.color).toBe('#ff0000');
    });
});
