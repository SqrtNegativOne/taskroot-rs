import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
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

function allInvokedArgs(command: string): Array<InvokeArgs | undefined> {
    return tauri.invoke.mock.calls
        .filter(([name]) => name === command)
        .map(([, args]) => args);
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

    it('sends the edited task description as a notes patch on blur', async () => {
        const task: AppTask = { id: 'task-1', title: 'Write docs' };
        tauri.stubCommandValue('query_tasks', [task]);
        render(InspectorPane, {
            props: {
                inspectorState: { type: 'task', id: task.id },
                onClose: vi.fn(),
            },
        });

        await fireEvent.click(await screen.findByRole('button', { name: 'Description' }));

        const textarea = await screen.findByRole('textbox', { name: 'Description' });
        await fireEvent.input(textarea, { target: { value: 'Revised notes' } });
        await fireEvent.blur(textarea);

        await waitFor(() => {
            expect(invokedArgs('update_task')).toEqual({
                id: 'task-1',
                patches: [{ field: 'notes', value: 'Revised notes' }],
            });
        });
    });

    it('sends separate non-clobbering patches when title and description are edited back-to-back', async () => {
        const task: AppTask = { id: 'task-1', title: 'Initial title', notes: 'Initial notes' };
        tauri.stubCommandValue('query_tasks', [task]);
        render(InspectorPane, {
            props: {
                inspectorState: { type: 'task', id: task.id },
                onClose: vi.fn(),
            },
        });

        const titleInput = await screen.findByRole('textbox', { name: 'Title' });
        await fireEvent.focus(titleInput);
        await fireEvent.input(titleInput, { target: { value: 'Renamed title' } });
        await fireEvent.blur(titleInput);

        await fireEvent.click(screen.getByRole('button', { name: 'Description' }));
        const descriptionInput = await screen.findByRole('textbox', { name: 'Description' });
        await fireEvent.input(descriptionInput, { target: { value: 'Updated notes' } });
        await fireEvent.blur(descriptionInput);

        await waitFor(() => {
            expect(allInvokedArgs('update_task')).toEqual([
                { id: 'task-1', patches: [{ field: 'title', value: 'Renamed title' }] },
                { id: 'task-1', patches: [{ field: 'notes', value: 'Updated notes' }] },
            ]);
        });
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it('applies quick-capture properties when adjusting a task title', async () => {
        vi.useFakeTimers({ toFake: ['Date'] });
        vi.setSystemTime(new Date(2026, 10, 19, 10, 0, 0));
        const calendars: AppCalendar[] = [
            { id: '/tasks/', summary: 'Tasks', writable: true, component: 'VTODO' },
            { id: '/work/', summary: 'Work', writable: true, component: 'VTODO' },
        ];
        const task: AppTask = {
            id: 'task-1',
            title: 'New Task',
            tags: [{ id: 'tag-1', name: 'existing' }],
            remoteCollectionId: '/tasks/',
        };
        tauri.stubCommandValue('query_tasks', [task]);
        render(InspectorPane, {
            props: {
                inspectorState: { type: 'task', id: task.id },
                calendars,
                onClose: vi.fn(),
            },
        });

        const titleInput = (await screen.findByRole('textbox', { name: 'Title' })) as HTMLTextAreaElement;
        await fireEvent.focus(titleInput);
        await fireEvent.input(titleInput, {
            target: { value: 'Ship release tomorrow at 9:30am #urgent p1 ^45m +work' },
        });
        expect(
            screen.getByText('Due Tomorrow 09:30 · #urgent · P4 · 45 min · Work'),
        ).not.toBeNull();

        await fireEvent.blur(titleInput);

        await waitFor(() => {
            expect(invokedArgs('update_task')).toEqual({
                id: 'task-1',
                patches: [
                    { field: 'title', value: 'Ship release' },
                    { field: 'due', value: '2026-11-20T09:30:00' },
                    { field: 'timezone', value: Intl.DateTimeFormat().resolvedOptions().timeZone },
                    { field: 'priority', value: 4 },
                    { field: 'est', value: 45 },
                    {
                        field: 'tags',
                        value: [
                            { id: 'tag-1', name: 'existing' },
                            expect.objectContaining({ name: 'urgent' }),
                        ],
                    },
                    { field: 'remoteCollectionId', value: '/work/' },
                ],
            });
        });
        expect(titleInput.value).toBe('Ship release');
    });

    it('keeps event titles literal instead of parsing task quick-capture sigils', async () => {
        tauri.stubCommandValue('query_events', [master]);
        render(InspectorPane, {
            props: {
                inspectorState: { type: 'event', id: master.id },
                onClose: vi.fn(),
            },
        });

        const titleInput = await screen.findByRole('textbox', { name: 'Title' });
        await fireEvent.focus(titleInput);
        await fireEvent.input(titleInput, { target: { value: 'Standup tomorrow #work p1' } });
        expect(screen.queryByRole('status')).toBeNull();
        await fireEvent.blur(titleInput);

        await waitFor(() => {
            expect(invokedArgs('update_event')).toEqual({
                id: 'master',
                patches: [{ field: 'title', value: 'Standup tomorrow #work p1' }],
            });
        });
    });

    it('sends event title and description edits as EventPatch arrays without duplicate calendar queries', async () => {
        tauri.stubCommandValue('query_events', [master]);
        render(InspectorPane, {
            props: {
                inspectorState: { type: 'event', id: master.id },
                onClose: vi.fn(),
            },
        });

        const titleInput = await screen.findByRole('textbox', { name: 'Title' });
        await fireEvent.focus(titleInput);
        await fireEvent.input(titleInput, { target: { value: 'Weekly Sync' } });
        await fireEvent.blur(titleInput);

        await fireEvent.click(screen.getByRole('button', { name: 'Description' }));
        const descriptionInput = await screen.findByRole('textbox', { name: 'Description' });
        await fireEvent.input(descriptionInput, { target: { value: 'Bring metrics' } });
        await fireEvent.blur(descriptionInput);

        await waitFor(() => {
            expect(allInvokedArgs('update_event')).toEqual([
                { id: 'master', patches: [{ field: 'title', value: 'Weekly Sync' }] },
                { id: 'master', patches: [{ field: 'description', value: 'Bring metrics' }] },
            ]);
        });
        expect(allInvokedArgs('get_active_calendars')).toHaveLength(1);
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
            expect(invokedArgs('delete_event')).toEqual({ id: 'single', scope: 'all', occurrenceStart: null });
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
