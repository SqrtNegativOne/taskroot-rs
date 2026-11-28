import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import DoScreen from './DoScreen.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

describe('DoScreen stub sections', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('query_tasks', []);
        tauri.stubCommandValue('get_stopwatch_state', {
            phase: 'idle',
            taskId: null,
            startedAt: null,
            accumulatedWorkMs: 0,
            breakDurationMs: 0,
            pausedUntil: null,
        });
    });

    it('does not read or write ui_state for placeholder collapsible stub sections', async () => {
        render(DoScreen);

        await fireEvent.click(screen.getByRole('button', { name: /distraction log/i }));
        await fireEvent.click(screen.getByRole('button', { name: /notes/i }));

        const uiStateCalls = tauri.invoke.mock.calls.filter(
            ([name]) => name === 'get_ui_state' || name === 'set_ui_state',
        );
        expect(uiStateCalls).toEqual([]);
    });
});
