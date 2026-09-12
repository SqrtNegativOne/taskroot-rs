import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../test/tauriMock';
import Harness from './persistedHarness.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();
const { invoke } = tauri;

const KEY = 'ui.test.filters';

function mockStoredSetting(value: unknown): void {
    tauri.stubCommandValue('get_ui_state', value);
}

function updateWrites(): unknown[][] {
    return invoke.mock.calls.filter(([cmd]) => cmd === 'set_ui_state');
}

describe('persistState', () => {
    beforeEach(() => {
        tauri.reset();
    });

    afterEach(() => {
        cleanup();
    });

    it('hydrates component state from the stored setting instead of the fallback', async () => {
        mockStoredSetting(['stored']);

        render(Harness, { props: { storageKey: KEY, fallback: ['fallback'] } });

        expect(await screen.findByText('stored')).toBeTruthy();
    });

    it('keeps the fallback when the stored setting is rejected by the validator', async () => {
        mockStoredSetting({ invalid: true });

        render(Harness, { props: { storageKey: KEY, fallback: ['fallback'], debounceMs: 5 } });

        await waitFor(() => {
            expect(invoke).toHaveBeenCalledWith('get_ui_state', { key: KEY });
        });
        await new Promise((resolve) => setTimeout(resolve, 20));
        expect(screen.getByTestId('value').textContent).toBe('fallback');
        expect(updateWrites()).toHaveLength(0);
    });

    it('does not write back a value that already matches the stored setting', async () => {
        mockStoredSetting(['stored']);

        render(Harness, { props: { storageKey: KEY, fallback: ['stored'], debounceMs: 5 } });

        await screen.findByText('stored');
        await new Promise((resolve) => setTimeout(resolve, 20));
        expect(updateWrites()).toHaveLength(0);
    });

    it('persists changes to the backend after the debounce window', async () => {
        mockStoredSetting(null);

        render(Harness, { props: { storageKey: KEY, fallback: ['fallback'], debounceMs: 5 } });
        await fireEvent.click(screen.getByText('add'));

        await waitFor(() => {
            expect(invoke).toHaveBeenCalledWith('set_ui_state', {
                key: KEY,
                value: ['fallback', 'added'],
            });
        });
    });

    it('coalesces rapid edits into a single write', async () => {
        mockStoredSetting(null);

        render(Harness, { props: { storageKey: KEY, fallback: [], debounceMs: 20 } });
        const addButton = screen.getByText('add');
        // Dispatch synchronously so hydration cannot interleave between clicks.
        void fireEvent.click(addButton);
        void fireEvent.click(addButton);
        void fireEvent.click(addButton);

        await waitFor(() => {
            expect(invoke).toHaveBeenCalledWith('set_ui_state', {
                key: KEY,
                value: ['added', 'added', 'added'],
            });
        });

        const writes = updateWrites();
        expect(writes).toHaveLength(1);
    });

    it('flushes a pending write when the component is destroyed', async () => {
        mockStoredSetting(null);

        const { unmount } = render(Harness, {
            props: { storageKey: KEY, fallback: [], debounceMs: 10_000 },
        });
        await waitFor(() => {
            expect(invoke).toHaveBeenCalledWith('get_ui_state', { key: KEY });
        });
        await new Promise((resolve) => setTimeout(resolve, 0));
        await fireEvent.click(screen.getByText('add'));

        unmount();

        expect(invoke).toHaveBeenCalledWith('set_ui_state', { key: KEY, value: ['added'] });
    });
});
