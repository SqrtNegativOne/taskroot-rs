import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import type { EventCallback } from '@tauri-apps/api/event';
import type { AppTask, StopwatchState as StopwatchSnapshot } from '../../../lib/domain';
import { STOPWATCH_UPDATED, STORE_UPDATED } from '../../../lib/ipcEvents';
import { store } from '../../../lib/store.svelte';
import { mockTauriInvoke } from '../../../test/tauriMock';
import { stopwatchState } from './engine.svelte';
import Stopwatch from './Stopwatch.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

vi.mock('@tauri-apps/api/window', () => ({
    getCurrentWindow: () => ({ label: 'main' }),
}));

const events = vi.hoisted(() => new Map<string, Set<EventCallback<StopwatchSnapshot>>>());

vi.mock('@tauri-apps/api/event', () => ({
    listen: (name: string, handler: EventCallback<StopwatchSnapshot>) => {
        const listeners = events.get(name) ?? new Set<EventCallback<StopwatchSnapshot>>();
        events.set(name, listeners);
        listeners.add(handler);
        return Promise.resolve(() => { listeners.delete(handler); });
    },
}));

const tauri = mockTauriInvoke();
const selectedTask: AppTask = { id: 'selected', title: 'Write the proposal' };
const otherTask: AppTask = { id: 'other', title: 'Review the budget' };

function snapshot(overrides: Partial<StopwatchSnapshot> = {}): StopwatchSnapshot {
    return {
        phase: 'paused',
        workElapsed: 0,
        workRunningSince: null,
        breakDuration: 0,
        breakStartedAt: null,
        pausedUntil: null,
        taskId: null,
        label: null,
        labelSource: 'unset',
        ...overrides,
    };
}

function publish(name: string, payload: StopwatchSnapshot): void {
    for (const handler of events.get(name) ?? []) {
        handler({ event: name, id: 1, payload });
    }
}

async function loadFocus(overrides: Partial<StopwatchSnapshot>): Promise<void> {
    tauri.stubCommandValue('get_stopwatch_state', snapshot(overrides));
    await stopwatchState.init();
}

beforeEach(async () => {
    tauri.reset();
    store.loaded = true;
    tauri.stubCommandValue('query_tasks', [selectedTask, otherTask]);
    await loadFocus({});
});

afterEach(() => {
store.loaded = false;
});

describe('Do screen shared focus', () => {
    it('shows the preselected task even while the clock is paused', async () => {
        await loadFocus({ taskId: selectedTask.id, label: 'Old title', labelSource: 'task' });
        render(Stopwatch);

        expect(await screen.findByText(selectedTask.title)).toBeTruthy();
        expect(screen.queryByRole('dialog')).toBeNull();
        expect(screen.queryByRole('button', { name: /task selector/i })).toBeNull();
        expect(screen.queryByRole('button', { name: 'Mini Tracker' })).toBeNull();
        expect(screen.queryByRole('button', { name: 'Take Break' })).toBeNull();
    });

    it('keeps the selected label visible while calendar tasks are loading', async () => {
        let resolveTasks: ((tasks: AppTask[]) => void) | undefined;
        tauri.stubCommand('query_tasks', () => new Promise<AppTask[]>((resolve) => {
            resolveTasks = resolve;
        }));
        await loadFocus({
            phase: 'running', taskId: selectedTask.id, label: selectedTask.title, labelSource: 'task',
        });
        render(Stopwatch);

        expect(await screen.findByText(selectedTask.title)).toBeTruthy();
        expect(screen.queryByRole('dialog')).toBeNull();
        if (resolveTasks === undefined) throw new Error('Query not started');
        resolveTasks([{ ...selectedTask, title: 'Updated proposal title' }]);
        expect(await screen.findByText('Updated proposal title')).toBeTruthy();
    });

    it('follows task, custom text and cleared focus updates from the mini tracker', async () => {
        render(Stopwatch);
        await waitFor(() => { expect(events.get(STOPWATCH_UPDATED)?.size).toBeGreaterThan(0); });

        publish(STOPWATCH_UPDATED, snapshot({
            taskId: selectedTask.id, label: selectedTask.title, labelSource: 'task',
        }));
        expect(await screen.findByText(selectedTask.title)).toBeTruthy();

        publish(STOPWATCH_UPDATED, snapshot({
            taskId: otherTask.id, label: otherTask.title, labelSource: 'task',
        }));
        expect(await screen.findByText(otherTask.title)).toBeTruthy();
        expect(screen.queryByText(selectedTask.title)).toBeNull();

        publish(STOPWATCH_UPDATED, snapshot({ label: 'Thinking time', labelSource: 'custom' }));
        expect(await screen.findByText('Thinking time')).toBeTruthy();
        expect(screen.queryByText(otherTask.title)).toBeNull();

        publish(STOPWATCH_UPDATED, snapshot());
        expect(await screen.findByText(/No task selected/)).toBeTruthy();
        expect(screen.queryByText('Thinking time')).toBeNull();
        expect(screen.queryByRole('dialog')).toBeNull();
    });

    it('refreshes the focused task title after calendar edits', async () => {
        await loadFocus({ taskId: selectedTask.id, label: selectedTask.title, labelSource: 'task' });
        render(Stopwatch);
        await screen.findByText(selectedTask.title);
        await waitFor(() => { expect(events.get(STORE_UPDATED)?.size).toBeGreaterThan(1); });

        tauri.stubCommandValue('query_tasks', [{ ...selectedTask, title: 'Renamed proposal' }]);
        publish(STORE_UPDATED, snapshot());
        expect(await screen.findByText('Renamed proposal')).toBeTruthy();
    });

    it('starts without a selection popup and preserves custom focus when toggling', async () => {
        const focused = snapshot({ label: 'Reading', labelSource: 'custom' });
        await loadFocus(focused);
        tauri.stubCommandValue('toggle_stopwatch', { ...focused, phase: 'running' });
        render(Stopwatch);

        await fireEvent.click(screen.getByRole('button', { name: 'Toggle stopwatch' }));
        await waitFor(() => { expect(stopwatchState.running).toBe(true); });
        expect(await screen.findByText('Reading')).toBeTruthy();
        expect(screen.queryByRole('dialog')).toBeNull();
    });

    it('keeps Space as a clock shortcut without opening a task selector', async () => {
        tauri.stubCommandValue('toggle_stopwatch', snapshot({ phase: 'running' }));
        render(Stopwatch);

        await fireEvent.keyDown(window, { code: 'Space' });
        await waitFor(() => { expect(stopwatchState.running).toBe(true); });
        await fireEvent.keyDown(window, { code: 'Enter', ctrlKey: true });
        expect(screen.queryByRole('dialog')).toBeNull();
    });
});

describe('Do screen clock shortcuts', () => {
    it.each(['input', 'textarea', 'div'])('does not intercept keys while editing a %s', async (tag) => {
        tauri.stubCommandValue('toggle_stopwatch', snapshot({ phase: 'running' }));
        render(Stopwatch);
        const editor = document.createElement(tag);
        if (tag === 'div') editor.setAttribute('contenteditable', 'true');
        document.body.append(editor);

        const allowedDefault = await fireEvent.keyDown(editor, { code: 'Space' });
        editor.remove();

        expect(allowedDefault).toBe(true);
        expect(stopwatchState.running).toBe(false);
        expect(tauri.invoke).not.toHaveBeenCalledWith('toggle_stopwatch', undefined);
    });

    it('toggles a timed pause and adjusts its duration with arrow keys', async () => {
        const paused = snapshot({ phase: 'running', pausedUntil: Date.now() + 60_000 });
        tauri.stubCommandValue('toggle_pause', paused);
        tauri.stubCommandValue('adjust_pause', paused);
        render(Stopwatch);

        expect(await fireEvent.keyDown(window, { code: 'KeyP' })).toBe(false);
        await waitFor(() => { expect(stopwatchState.isPaused).toBe(true); });
        expect(tauri.invoke).toHaveBeenCalledWith('toggle_pause', { pauseMinutes: 10 });

        expect(await fireEvent.keyDown(window, { code: 'ArrowRight' })).toBe(false);
        expect(tauri.invoke).toHaveBeenCalledWith('adjust_pause', { deltaMinutes: 1 });
        expect(await fireEvent.keyDown(window, { code: 'ArrowDown' })).toBe(false);
        expect(tauri.invoke).toHaveBeenCalledWith('adjust_pause', { deltaMinutes: -1 });
    });

    it('leaves arrow keys alone outside a timed pause', async () => {
        render(Stopwatch);

        expect(await fireEvent.keyDown(window, { code: 'ArrowUp' })).toBe(true);
        expect(tauri.invoke).not.toHaveBeenCalledWith('adjust_pause', expect.anything());
    });

    it('resets through Ctrl+R and stops handling shortcuts after unmount', async () => {
        await loadFocus({ workElapsed: 60_000 });
        tauri.stubCommandValue('reset_stopwatch', snapshot());
        tauri.stubCommandValue('toggle_stopwatch', snapshot({ phase: 'running' }));
        const { unmount } = render(Stopwatch);

        expect(await fireEvent.keyDown(window, { code: 'KeyR', ctrlKey: true })).toBe(false);
        await waitFor(() => { expect(stopwatchState.workMs).toBe(0); });
        unmount();
        expect(await fireEvent.keyDown(window, { code: 'Space' })).toBe(true);
        expect(stopwatchState.running).toBe(false);
        expect(tauri.invoke).not.toHaveBeenCalledWith('toggle_stopwatch', undefined);
    });
});
