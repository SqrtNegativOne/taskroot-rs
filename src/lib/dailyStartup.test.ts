import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mockTauriInvoke } from '../test/tauriMock';
import { claimFirstOpenOfDay, isFirstOpenToday } from './dailyStartup';
import { persistedKeys } from './persisted.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

describe('isFirstOpenToday', () => {
    it('is false when the stored date is today', () => {
        expect(isFirstOpenToday('2026-11-22', '2026-11-22')).toBe(false);
    });

    it('is true when the stored date is missing or a different day', () => {
        expect(isFirstOpenToday(null, '2026-11-22')).toBe(true);
        expect(isFirstOpenToday('2026-11-21', '2026-11-22')).toBe(true);
    });
});

describe('claimFirstOpenOfDay', () => {
    beforeEach(() => {
        tauri.reset();
    });

    it('records today and claims the first open when nothing was stored', async () => {
        tauri.stubCommandValue('get_ui_state', null);

        const claimed = await claimFirstOpenOfDay(new Date(2026, 10, 22, 8, 30));

        expect(claimed).toBe(true);
        expect(tauri.invoke).toHaveBeenCalledWith('set_ui_state', {
            key: persistedKeys.appLastOpened,
            value: '2026-11-22',
        });
    });

    it('does not claim or write again on a later launch the same day', async () => {
        tauri.stubCommandValue('get_ui_state', '2026-11-22');

        const claimed = await claimFirstOpenOfDay(new Date(2026, 10, 22, 18, 0));

        expect(claimed).toBe(false);
        expect(tauri.invoke).not.toHaveBeenCalledWith('set_ui_state', expect.anything());
    });

    it('claims again on the next day', async () => {
        tauri.stubCommandValue('get_ui_state', '2026-11-22');

        const claimed = await claimFirstOpenOfDay(new Date(2026, 10, 23, 7, 0));

        expect(claimed).toBe(true);
        expect(tauri.invoke).toHaveBeenCalledWith('set_ui_state', {
            key: persistedKeys.appLastOpened,
            value: '2026-11-23',
        });
    });

    it('leaves the default route in place when the read fails', async () => {
        tauri.stubCommand('get_ui_state', () => Promise.reject(new Error('offline')));

        const claimed = await claimFirstOpenOfDay(new Date(2026, 10, 22, 9, 0));

        expect(claimed).toBe(false);
        expect(tauri.invoke).not.toHaveBeenCalledWith('set_ui_state', expect.anything());
    });

    it('does not claim the day when recording it fails', async () => {
        tauri.stubCommandValue('get_ui_state', null);
        tauri.stubCommand('set_ui_state', () => Promise.reject(new Error('offline')));

        const claimed = await claimFirstOpenOfDay(new Date(2026, 10, 22, 9, 0));

        expect(claimed).toBe(false);
    });
});
