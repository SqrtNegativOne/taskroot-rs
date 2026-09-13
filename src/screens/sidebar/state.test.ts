import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import { readOrSeedSidebar, sidebarSeeds, sidebarUiKeys } from './state.svelte';
import Harness from './sidebarStateHarness.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();
const { invoke } = tauri;

function updateWrites(): unknown[][] {
    return invoke.mock.calls.filter(([cmd]) => cmd === 'set_ui_state');
}

function stubBackend(values: Record<string, unknown>): void {
    tauri.stubCommand('get_ui_state', (args) => {
        const key = (args as { key: string }).key;
        return values[key] ?? null;
    });
}

describe('readOrSeedSidebar', () => {
    beforeEach(() => {
        tauri.reset();
        localStorage.clear();
    });

    it('seeds the backend from a legacy localStorage value and removes the legacy key', async () => {
        stubBackend({});
        localStorage.setItem('sidebar_notes', 'legacy notes');

        const value = await readOrSeedSidebar(sidebarSeeds.notes);

        expect(value).toBe('legacy notes');
        expect(invoke).toHaveBeenCalledWith('set_ui_state', {
            key: sidebarUiKeys.notes,
            value: 'legacy notes',
        });
        expect(localStorage.getItem('sidebar_notes')).toBeNull();
    });

    it('lets a backend value win and never consults localStorage', async () => {
        stubBackend({ [sidebarUiKeys.notes]: 'backend notes' });
        localStorage.setItem('sidebar_notes', 'legacy notes');
        const getItem = vi.spyOn(Storage.prototype, 'getItem');

        const value = await readOrSeedSidebar(sidebarSeeds.notes);

        expect(value).toBe('backend notes');
        expect(getItem).not.toHaveBeenCalled();
        expect(updateWrites()).toHaveLength(0);
    });

    it('ignores an unparseable legacy tab position', async () => {
        stubBackend({});
        localStorage.setItem('sidebar_tab_top', 'not-a-number');

        const value = await readOrSeedSidebar(sidebarSeeds.tabTop);

        expect(value).toBeUndefined();
        expect(updateWrites()).toHaveLength(0);
        expect(localStorage.getItem('sidebar_tab_top')).toBeNull();
    });

    it('parses a numeric legacy tab position', async () => {
        stubBackend({});
        localStorage.setItem('sidebar_tab_top', '123.5');

        const value = await readOrSeedSidebar(sidebarSeeds.tabTop);

        expect(value).toBe(123.5);
        expect(invoke).toHaveBeenCalledWith('set_ui_state', {
            key: sidebarUiKeys.tabTop,
            value: 123.5,
        });
    });
});

describe('createSidebarState', () => {
    beforeEach(() => {
        tauri.reset();
        localStorage.clear();
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
