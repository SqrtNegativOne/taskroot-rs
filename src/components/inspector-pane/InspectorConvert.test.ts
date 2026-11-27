import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke, type InvokeArgs } from '../../test/tauriMock';
import { makeEvent } from '../../test/factories';
import type { AppCalendar, AppTask } from '../../lib/domain';
import InspectorPane from './InspectorPane.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

function invokedArgs(command: string): InvokeArgs | undefined {
    return tauri.invoke.mock.calls.find(([name]) => name === command)?.[1];
}

function createdTask(): AppTask | undefined {
    const args = invokedArgs('create_task');
    return args === undefined ? undefined : (args.task as AppTask);
}

const calendars: AppCalendar[] = [
    { id: 'cal-events', summary: 'Event Calendar', writable: true, component: 'VEVENT' },
    { id: 'cal-tasks', summary: 'Task List', writable: true, component: 'VTODO' },
];

describe('InspectorPane convert to task', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('query_tasks', []);
        tauri.stubCommandValue('get_active_calendars', calendars);
        tauri.stubCommandValue('get_ui_state', undefined);
    });

    it('creates a task from the event date and removes the event', async () => {
        const event = makeEvent({
            id: 'event-1',
            title: 'Design review',
            description: 'Bring the mockups',
            startTime: '2026-10-19T11:00:00',
            endTime: '2026-10-19T12:00:00',
            priority: 1,
            tags: [{ id: 'tag-1', name: 'work' }],
        });
        tauri.stubCommandValue('query_events', [event]);
        const onConverted = vi.fn();

        render(InspectorPane, {
            props: {
                inspectorState: { type: 'event', id: 'event-1' },
                onClose: vi.fn(),
                onConverted,
            },
        });

        await fireEvent.click(await screen.findByRole('button', { name: /convert to task/i }));

        await waitFor(() => {
            expect(invokedArgs('delete_event')).toEqual({
                id: 'event-1',
                scope: 'all',
                occurrenceStart: undefined,
            });
        });

        expect(createdTask()).toMatchObject({
            title: 'Design review',
            notes: 'Bring the mockups',
            start: '2026-10-19',
            status: 'todo',
            priority: 1,
            remoteCollectionId: 'cal-tasks',
            tags: [{ name: 'work' }],
        });
        expect(onConverted).toHaveBeenCalledWith({
            type: 'task',
            id: createdTask()?.id,
            draft: createdTask(),
        });
    });

    it('uses the clicked occurrence date and scope for a recurring event', async () => {
        const master = makeEvent({
            id: 'master',
            title: 'Standup',
            startTime: '2026-10-19T11:00:00',
            endTime: '2026-10-19T11:30:00',
            rrule: 'RRULE:FREQ=WEEKLY',
        });
        tauri.stubCommandValue('query_events', [master]);
        const onConverted = vi.fn();

        render(InspectorPane, {
            props: {
                inspectorState: {
                    type: 'event',
                    id: 'master',
                    occurrenceStart: '2026-10-26T11:00:00',
                },
                onClose: vi.fn(),
                onConverted,
            },
        });

        await fireEvent.click(await screen.findByRole('button', { name: /convert to task/i }));

        expect(screen.getByRole('dialog')).toBeTruthy();
        await fireEvent.click(screen.getByLabelText('This and following events'));
        await fireEvent.click(screen.getByRole('button', { name: 'OK' }));

        await waitFor(() => {
            expect(invokedArgs('delete_event')).toEqual({
                id: 'master',
                scope: 'following',
                occurrenceStart: '2026-10-26T11:00:00',
            });
        });

        expect(createdTask()).toMatchObject({ start: '2026-10-26' });
        expect(onConverted).toHaveBeenCalledOnce();
    });

    it('does not convert when no writable task list exists', async () => {
        tauri.stubCommandValue('get_active_calendars', [
            { id: 'cal-events', summary: 'Event Calendar', writable: true, component: 'VEVENT' },
        ]);
        tauri.stubCommandValue('query_events', [makeEvent({ id: 'event-1' })]);
        const onConverted = vi.fn();

        render(InspectorPane, {
            props: {
                inspectorState: { type: 'event', id: 'event-1' },
                onClose: vi.fn(),
                onConverted,
            },
        });

        await fireEvent.click(await screen.findByRole('button', { name: /convert to task/i }));

        expect(invokedArgs('create_task')).toBeUndefined();
        expect(invokedArgs('delete_event')).toBeUndefined();
        expect(onConverted).not.toHaveBeenCalled();
    });
});
