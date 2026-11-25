import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import type { StopwatchState as StopwatchSnapshot } from '../../../lib/domain';
import { SCREEN_EFFECT } from '../../../lib/ipcEvents';
import { mockTauriInvoke } from '../../../test/tauriMock';
import {
    StopwatchState,
    stopwatchDisplay,
    stopwatchState,
    type StopwatchDisplaySource,
} from './engine.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const runtime = vi.hoisted(() => ({
    label: 'main',
    now: 1_000_000,
    emit: vi.fn(),
    listen: vi.fn(() => Promise.resolve(() => { /* noop */ })),
}));

vi.mock('@tauri-apps/api/window', () => ({
    getCurrentWindow: () => ({ label: runtime.label }),
}));

vi.mock('@tauri-apps/api/event', () => ({
    listen: runtime.listen,
    emit: runtime.emit,
}));

vi.mock('../../../lib/useNow.svelte', () => ({
    useNow: () => ({ get ms() { return runtime.now; } }),
}));

const tauri = mockTauriInvoke();
const playedSounds: string[] = [];

function snapshot(overrides: Partial<StopwatchSnapshot> = {}): StopwatchSnapshot {
    return {
        phase: 'paused', workElapsed: 0, workRunningSince: null,
        breakDuration: 0, breakStartedAt: null, pausedUntil: null,
        taskId: null, label: null, labelSource: 'unset',
        ...overrides,
    };
}

function apply(overrides: Partial<StopwatchSnapshot>): void {
    flushSync(() => { stopwatchState.updateFromPayload(snapshot(overrides)); });
}

beforeEach(async () => {
    runtime.label = 'main';
    runtime.now = 1_000_000;
    tauri.reset();
    runtime.emit.mockClear();
    runtime.listen.mockReset();
    runtime.listen.mockImplementation(() => Promise.resolve(() => { /* noop */ }));
    vi.spyOn(HTMLMediaElement.prototype, 'play').mockImplementation(function (this: HTMLMediaElement) {
        playedSounds.push(new URL(this.src).pathname);
        return Promise.resolve();
    });
    const wasInBreak = stopwatchState.isBreak;
    tauri.stubCommandValue('get_stopwatch_state', snapshot());
    await stopwatchState.init();
    flushSync();
    if (wasInBreak) {
        await vi.waitFor(() => { expect(runtime.emit).toHaveBeenCalledWith(SCREEN_EFFECT); });
    }
    tauri.reset();
    runtime.emit.mockClear();
    playedSounds.length = 0;
});

describe('stopwatch clock projection', () => {
    it('adds running time to the stored elapsed work without changing focus', () => {
        apply({
            phase: 'running', workElapsed: 30_000, workRunningSince: runtime.now - 5_000,
            taskId: 'proposal', label: 'Write proposal', labelSource: 'task',
        });

        expect(stopwatchState.currentMs).toBe(35_000);
        expect(stopwatchState.taskId).toBe('proposal');
        expect(stopwatchState.label).toBe('Write proposal');
        runtime.now += 2_000;
        expect(stopwatchState.workMs).toBe(37_000);
        expect(playedSounds).toEqual([]);
    });

    it('shows the timed pause then resumes work from its deadline', () => {
        apply({ phase: 'running', workElapsed: 30_000, pausedUntil: runtime.now + 60_000 });

        expect(stopwatchState.isPaused).toBe(true);
        expect(stopwatchState.currentMs).toBe(60_000);
        runtime.now += 65_000;
        expect(stopwatchState.isPaused).toBe(false);
        expect(stopwatchState.currentMs).toBe(35_000);
    });
});

describe('stopwatch display', () => {
    function source(overrides: Partial<StopwatchDisplaySource> = {}): StopwatchDisplaySource {
        return {
            isPaused: false,
            pauseRemainingMs: 0,
            isBreak: false,
            isCountdown: false,
            breakRemainingMs: 0,
            breakElapsedMs: 0,
            isStopped: false,
            workMs: 0,
            isPristine: false,
            ...overrides,
        };
    }

    it('shows whole minutes while working', () => {
        expect(stopwatchDisplay(source({ workMs: 125_000 })).primaryText).toBe('02');
    });

    it('shows total elapsed seconds during a break', () => {
        expect(stopwatchDisplay(source({
            isBreak: true, breakElapsedMs: 65_000, breakRemainingMs: 0,
        })).primaryText).toBe('65');
    });

    it('shows remaining seconds for a countdown break', () => {
        expect(stopwatchDisplay(source({
            isBreak: true, isCountdown: true, breakElapsedMs: 15_000, breakRemainingMs: 45_000,
        })).primaryText).toBe('45');
    });

});

describe('stopwatch feedback', () => {
    it('announces entering a break once, not on every break update', async () => {
        apply({ phase: 'breaking', breakStartedAt: runtime.now, breakDuration: 60_000 });
        apply({ phase: 'breaking', breakStartedAt: runtime.now, breakDuration: 120_000 });

        expect(playedSounds).toEqual(['/sounds/work_to_break.wav']);
        await vi.waitFor(() => { expect(runtime.emit).toHaveBeenCalledWith(SCREEN_EFFECT); });
        expect(runtime.emit).toHaveBeenCalledTimes(1);
    });

    it('announces leaving a break for work with the transition overlay', async () => {
        apply({ phase: 'breaking', breakStartedAt: runtime.now, breakDuration: 60_000 });
        await vi.waitFor(() => { expect(runtime.emit).toHaveBeenCalledTimes(1); });
        playedSounds.length = 0;
        runtime.emit.mockClear();

        apply({ phase: 'running', workRunningSince: runtime.now });

        expect(playedSounds).toEqual(['/sounds/break_to_work.wav']);
        await vi.waitFor(() => { expect(runtime.emit).toHaveBeenCalledWith(SCREEN_EFFECT); });
    });

    it('rings the break-over alarm without an overlay when the break stops', async () => {
        apply({ phase: 'breaking', breakStartedAt: runtime.now, breakDuration: 60_000 });
        await vi.waitFor(() => { expect(runtime.emit).toHaveBeenCalledTimes(1); });
        playedSounds.length = 0;
        runtime.emit.mockClear();
        tauri.reset();

        apply({ phase: 'stopped' });

        expect(stopwatchState.currentMs).toBe(0);
        expect(playedSounds).toEqual(['/sounds/wine-glass-alarm.ogg']);
        expect(tauri.invoke).not.toHaveBeenCalledWith('show_screen_effect', undefined);
        expect(runtime.emit).not.toHaveBeenCalled();
    });

    it('does not duplicate feedback in a companion window', async () => {
        runtime.label = 'minitracker';
        apply({ phase: 'breaking', breakStartedAt: runtime.now, breakDuration: 60_000 });
        await Promise.resolve();

        expect(playedSounds).toEqual([]);
        expect(runtime.emit).not.toHaveBeenCalled();
        expect(tauri.invoke).not.toHaveBeenCalledWith('show_screen_effect', undefined);
    });

    it('does not emit an overlay event when opening the effect window fails', async () => {
        tauri.stubCommand('show_screen_effect', () => Promise.reject(new Error('unavailable')));
        apply({ phase: 'breaking', breakStartedAt: runtime.now, breakDuration: 60_000 });
        await vi.dynamicImportSettled();

        expect(tauri.invoke).toHaveBeenCalledWith('show_screen_effect', undefined);
        expect(playedSounds).toEqual(['/sounds/work_to_break.wav']);
        expect(runtime.emit).not.toHaveBeenCalled();
    });

    it('plays the transport error sound and leaves the clock untouched on failure', async () => {
        apply({ workElapsed: 30_000, label: 'Reading', labelSource: 'custom' });
        tauri.stubCommand('toggle_stopwatch', () => Promise.reject(new Error('unavailable')));

        await stopwatchState.toggle();

        expect(stopwatchState.phase).toBe('paused');
        expect(stopwatchState.currentMs).toBe(30_000);
        expect(stopwatchState.label).toBe('Reading');
        expect(playedSounds).toEqual(['/sounds/windows-xp-error.mp3']);
    });
});

describe('stopwatch task selection', () => {
    it('does not start the clock when selecting the task fails', async () => {
        apply({ phase: 'paused' });
        tauri.stubCommand('set_stopwatch_task', () => Promise.reject(new Error('unavailable')));

        await stopwatchState.startTask({ id: 'proposal', title: 'Write proposal' });

        expect(tauri.invoke).not.toHaveBeenCalledWith('toggle_stopwatch', undefined);
        expect(stopwatchState.phase).toBe('paused');
    });

    it('starts the clock once the task is selected', async () => {
        apply({ phase: 'paused' });
        tauri.stubCommandValue('set_stopwatch_task', snapshot({
            taskId: 'proposal', label: 'Write proposal', labelSource: 'task',
        }));
        tauri.stubCommandValue('toggle_stopwatch', snapshot({
            phase: 'running', workRunningSince: runtime.now,
        }));

        await stopwatchState.startTask({ id: 'proposal', title: 'Write proposal' });

        expect(tauri.invoke).toHaveBeenCalledWith('toggle_stopwatch', undefined);
        expect(stopwatchState.phase).toBe('running');
    });
});

describe('stopwatch engine lifecycle', () => {
    it('unregisters a listener that resolves after disposal', async () => {
        const deferred = Promise.withResolvers<() => void>();
        runtime.listen.mockImplementationOnce(() => deferred.promise);
        const unlisten = vi.fn();

        const engine = new StopwatchState();
        await vi.waitFor(() => { expect(runtime.listen).toHaveBeenCalled(); });
        engine.dispose();
        deferred.resolve(unlisten);

        await vi.waitFor(() => { expect(unlisten).toHaveBeenCalled(); });
    });
});
