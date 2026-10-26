import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import { sidebarUiKeys } from './state.svelte';
import Harness from './sidebarStateHarness.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();
const { invoke } = tauri;

function stubBackend(values: Record<string, unknown>): void {
    tauri.stubCommand('get_ui_state', (args) => {
        const key = (args as { key: string }).key;
        return values[key] ?? null;
    });
}

describe('createSidebarState', () => {
    beforeEach(() => {
        tauri.reset();
    });

    afterEach(() => {
        cleanup();
    });

    it('hydrates notes, visibility and tab position from the backend', async () => {
        stubBackend({
            [sidebarUiKeys.notes]: 'from backend',
            [sidebarUiKeys.showNotes]: true,
            [sidebarUiKeys.tabTop]: 12,
        });

        render(Harness);

        await waitFor(() => {
            expect(screen.getByTestId('hydrated').textContent).toBe('true');
        });
        expect(screen.getByTestId('notes').textContent).toBe('from backend');
        expect(screen.getByTestId('show-notes').textContent).toBe('true');
        expect(screen.getByTestId('tab-top').textContent).toBe('12');
    });

    it('round-trips notes and tab position back to the backend', async () => {
        stubBackend({});
        render(Harness);
        await waitFor(() => {
            expect(screen.getByTestId('hydrated').textContent).toBe('true');
        });

        await fireEvent.click(screen.getByText('edit-notes'));
        await waitFor(() => {
            expect(invoke).toHaveBeenCalledWith('set_ui_state', {
                key: sidebarUiKeys.notes,
                value: '!',
            });
        });

        await fireEvent.click(screen.getByText('save-tab'));
        expect(invoke).toHaveBeenCalledWith('set_ui_state', {
            key: sidebarUiKeys.tabTop,
            value: 42,
        });
    });
});
