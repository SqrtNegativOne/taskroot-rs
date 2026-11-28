import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import { makeEvent } from '../../test/factories';
import type { AppCalendar, AppEvent, AppTask, EventPatch } from '../../lib/domain';
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
    const updateEvent = vi.fn<(id: string, patches: EventPatch[]) => void>();
    render(InspectorEventFields, {
        props: { event, tasks: [], calendars, updateEvent },
    });
    return updateEvent;
}

describe('inspector calendar options', () => {
    beforeEach(() => {
        tauri.reset();
    });

    it('offers only event calendars to an event without querying get_active_calendars', async () => {
        renderEventFields(makeEvent({ remoteCollectionId: 'cal-events' }));

        await fireEvent.click(await screen.findByRole('button', { name: 'Calendar' }));

        expect(screen.getByRole('button', { name: 'Event Calendar' })).toBeTruthy();
        expect(screen.queryByRole('button', { name: 'Task List' })).toBeNull();
        expect(tauri.invoke).not.toHaveBeenCalledWith('get_active_calendars', expect.anything(), expect.anything());
    });

    it('offers only task lists to a task without querying get_active_calendars', async () => {
        const task: AppTask = {
            id: 'task-1',
            title: 'Task',
            remoteCollectionId: 'cal-tasks',
        };

        render(InspectorTaskFields, {
            props: { task, tasks: [task], calendars, updateTask: vi.fn() },
        });

        await fireEvent.click(await screen.findByRole('button', { name: 'List' }));

        expect(screen.getByRole('button', { name: 'Task List' })).toBeTruthy();
        expect(screen.queryByRole('button', { name: 'Event Calendar' })).toBeNull();
        expect(tauri.invoke).not.toHaveBeenCalledWith('get_active_calendars', expect.anything(), expect.anything());
    });
});

describe('event colour field', () => {
    beforeEach(() => {
        tauri.reset();
    });

    it('inherits the calendar colour when the event has no override', async () => {
        renderEventFields(makeEvent({ remoteCollectionId: 'cal-events' }));

        const input = (await screen.findByLabelText('Event colour')) as HTMLInputElement;
        await waitFor(() => expect(input.value).toBe('#123456'));
        expect(screen.getByText('From calendar')).toBeTruthy();
        expect(screen.queryByRole('button', { name: 'Clear event colour' })).toBeNull();
    });

    it('writes a custom override and clears it back to calendar inheritance', async () => {
        const event = makeEvent({ id: 'ev-1', remoteCollectionId: 'cal-events' });
        const updateEvent = renderEventFields(event);
        const input = await screen.findByLabelText('Event colour');

        await fireEvent.change(input, { target: { value: '#ff0000' } });
        expect(updateEvent).toHaveBeenLastCalledWith('ev-1', [{ field: 'color', value: '#ff0000' }]);

        const overridden = makeEvent({ id: 'ev-2', remoteCollectionId: 'cal-events', color: '#ff0000' });
        const updateOverridden = renderEventFields(overridden);
        await fireEvent.click(await screen.findByRole('button', { name: 'Clear event colour' }));
        expect(updateOverridden).toHaveBeenLastCalledWith('ev-2', [{ field: 'color', value: null }]);
    });

    it('emits an atomic date patch bundle when toggling all-day', async () => {
        const event = makeEvent({
            id: 'ev-timed',
            startTime: '2026-10-19T11:00:00',
            endTime: '2026-10-19T12:00:00',
            isAllDay: false,
        });
        const updateEvent = renderEventFields(event);

        await fireEvent.click(await screen.findByRole('button', { name: 'When' }));
        await fireEvent.click(await screen.findByRole('switch', { name: 'All day event' }));
        expect(updateEvent).toHaveBeenLastCalledWith('ev-timed', [
            { field: 'startTime', value: '2026-10-19' },
            { field: 'endTime', value: '2026-10-20' },
            { field: 'isAllDay', value: true },
        ]);
    });
});
