import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import { makeEvent } from '../../test/factories';
import type { AppCalendar, AppEvent, AppTask } from '../../lib/domain';
import InspectorEventFields from './InspectorEventFields.svelte';
import InspectorTaskFields from './InspectorTaskFields.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

const calendars: AppCalendar[] = [
    { id: 'cal-events', summary: 'Event Calendar', writable: true, component: 'VEVENT', color: '#123456' },
    { id: 'cal-other', summary: 'Other Calendar', writable: true, component: 'VEVENT', color: '#654321' },
    { id: 'cal-tasks', summary: 'Task List', writable: true, component: 'VTODO' },
];

function renderEventFields(event: AppEvent) {
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

describe('inspector calendar options', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('get_active_calendars', calendars);
    });

    it('offers only event calendars to an event', async () => {
        renderEventFields(makeEvent({ remoteCollectionId: 'cal-events' }));

        await fireEvent.click(await screen.findByRole('button', { name: 'Calendar' }));

        expect(screen.getByRole('button', { name: 'Event Calendar' })).toBeTruthy();
        expect(screen.queryByRole('button', { name: 'Task List' })).toBeNull();
    });

    it('offers only task lists to a task', async () => {
        const task: AppTask = {
            id: 'task-1',
            title: 'Task',
            remoteCollectionId: 'cal-tasks',
        };

        render(InspectorTaskFields, {
            props: { task, tasks: [task], updateTask: vi.fn() },
        });

        await fireEvent.click(await screen.findByRole('button', { name: 'List' }));

        expect(screen.getByRole('button', { name: 'Task List' })).toBeTruthy();
        expect(screen.queryByRole('button', { name: 'Event Calendar' })).toBeNull();
    });
});

describe('event colour field', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('get_active_calendars', calendars);
    });

    it('inherits the calendar colour when the event has no override', async () => {
        renderEventFields(makeEvent({ remoteCollectionId: 'cal-events' }));

        const input = (await screen.findByLabelText('Event colour')) as HTMLInputElement;
        await waitFor(() => expect(input.value).toBe('#123456'));
        expect(screen.getByText('From calendar')).toBeTruthy();
        expect(screen.queryByRole('button', { name: 'Clear event colour' })).toBeNull();
    });

    it('writes a custom override and clears it back to calendar inheritance', async () => {
        const event = makeEvent({ remoteCollectionId: 'cal-events' });
        const updateEvent = renderEventFields(event);
        const input = await screen.findByLabelText('Event colour');

        await fireEvent.change(input, { target: { value: '#ff0000' } });
        expect(lastUpdate(updateEvent, event).color).toBe('#ff0000');

        const overridden = makeEvent({ remoteCollectionId: 'cal-events', color: '#ff0000' });
        const updateOverridden = renderEventFields(overridden);
        await fireEvent.click(await screen.findByRole('button', { name: 'Clear event colour' }));
        expect(lastUpdate(updateOverridden, overridden).color).toBeUndefined();
    });
});
