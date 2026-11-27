import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import { fireEvent } from '@testing-library/svelte';
import { mockTauriInvoke, type InvokeArgs } from '../../test/tauriMock';
import type { AppTask } from '../../lib/domain';
import InspectorPane from './InspectorPane.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

function invokedArgs(command: string): InvokeArgs | undefined {
    return tauri.invoke.mock.calls.find(([name]) => name === command)?.[1];
}

const task: AppTask = { id: 'task-1', title: 'Write docs' };

function renderTaskInspector() {
    return render(InspectorPane, {
        props: {
            inspectorState: { type: 'task', id: task.id },
            onClose: vi.fn(),
        },
    });
}

describe('inspector description persistence', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('query_tasks', [task]);
        tauri.stubCommandValue('query_events', []);
        tauri.stubCommandValue('get_active_calendars', []);
        tauri.stubCommandValue('get_ui_state', undefined);
    });

    it('sends the edited description to update_task', async () => {
        renderTaskInspector();

        await fireEvent.click(await screen.findByRole('button', { name: 'Description' }));

        const textarea = await screen.findByRole('textbox', { name: 'Description' });
        await fireEvent.input(textarea, { target: { value: 'Revised notes' } });
        await fireEvent.blur(textarea);

        await waitFor(() => {
            const args = invokedArgs('update_task');
            expect((args?.['task'] as AppTask | undefined)?.notes).toBe('Revised notes');
        });
    });
});
