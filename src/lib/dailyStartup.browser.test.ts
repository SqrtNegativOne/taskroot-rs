import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mockTauriInvoke } from '../test/tauriMock';
import { claimFirstOpenOfDay } from './dailyStartup';
import { persistedKeys } from './persisted.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

describe('claimFirstOpenOfDay', () => {
    beforeEach(() => {
        tauri.reset();
    });

    it('does not claim or read state before the 4:30 AM daily cutoff', async () => {
        tauri.stubCommandValue('get_ui_state', null);

        const claimed = await claimFirstOpenOfDay(new Date(2026, 10, 22, 4, 29));

        expect(claimed).toBe(false);
        expect(tauri.invoke).not.toHaveBeenCalled();
    });

    it('claims at or after 4:30 AM and records today so later launches skip it', async () => {
        tauri.stubCommandValue('get_ui_state', '2026-11-21');

        expect(await claimFirstOpenOfDay(new Date(2026, 10, 22, 4, 30))).toBe(true);
        expect(tauri.invoke).toHaveBeenCalledWith(
            'set_ui_state',
            { key: persistedKeys.appLastOpened, value: '2026-11-22' },
            undefined,
        );

        tauri.reset();
        tauri.stubCommandValue('get_ui_state', '2026-11-22');

        expect(await claimFirstOpenOfDay(new Date(2026, 10, 22, 18, 0))).toBe(false);
        expect(tauri.invoke).not.toHaveBeenCalledWith('set_ui_state', expect.anything(), undefined);
    });

    it('falls back without claiming when reading or writing fails', async () => {
        tauri.stubCommand('get_ui_state', () => Promise.reject(new Error('offline')));
        expect(await claimFirstOpenOfDay(new Date(2026, 10, 22, 9, 0))).toBe(false);
        expect(tauri.invoke).not.toHaveBeenCalledWith('set_ui_state', expect.anything(), undefined);

        tauri.reset();
        tauri.stubCommandValue('get_ui_state', null);
        tauri.stubCommand('set_ui_state', () => Promise.reject(new Error('offline')));
        expect(await claimFirstOpenOfDay(new Date(2026, 10, 22, 9, 0))).toBe(false);
    });
});
