import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import { makeEvent } from '../../test/factories';
import type { AppCalendar, AppTask } from '../../lib/domain';
import InspectorEventFields from './InspectorEventFields.svelte';
import InspectorTaskFields from './InspectorTaskFields.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

const calendars: AppCalendar[] = [
    { id: 'cal-events', summary: 'Event Calendar', writable: true, component: 'VEVENT' },
    { id: 'cal-tasks', summary: 'Task List', writable: true, component: 'VTODO' },
];

describe('inspector calendar options', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('get_active_calendars', calendars);
    });

    afterEach(() => {
        cleanup();
    });

    it('offers only event calendars to an event', async () => {
        render(InspectorEventFields, {
            props: {
                event: makeEvent({ remoteCollectionId: 'cal-events' }),
                tasks: [],
                updateEvent: vi.fn(),
            },
        });

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
            props: {
                task,
                tasks: [task],
                updateTask: vi.fn(),
            },
        });

        await fireEvent.click(await screen.findByRole('button', { name: 'List' }));

        expect(screen.getByRole('button', { name: 'Task List' })).toBeTruthy();
        expect(screen.queryByRole('button', { name: 'Event Calendar' })).toBeNull();
    });
});
