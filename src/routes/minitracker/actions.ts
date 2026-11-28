import { DEFAULT_TASK_PRIORITY, type AppTask, type AppTaskStatus } from '../../lib/domain';
import { commands, safeInvoke } from '../../lib/safeInvoke.svelte';
import { store } from '../../lib/store.svelte';
import { stopwatchState } from '../../screens/do/stopwatch/engine.svelte';
import type { MinitrackerKeyCommand } from './shortcuts';

export interface MinitrackerActionDeps {
    readonly closeMenu: () => Promise<void>;
    readonly getPauseMinutes: () => number;
    readonly getBreakDivisor: () => number;
    readonly toggleDimmed: () => void;
}

export function createMinitrackerActions(deps: MinitrackerActionDeps) {
    async function setTaskStatus(task: AppTask, status: AppTaskStatus): Promise<void> {
        if (task.status === status) return;
        await store.mutate('Failed to update task status', () =>
            store.updateTask(task.id, [{ field: 'status', value: status }]));
    }

    async function stopClock(): Promise<void> {
        if (stopwatchState.isPaused) await stopwatchState.togglePause(deps.getPauseMinutes());
        if (stopwatchState.running && !stopwatchState.isBreak) await stopwatchState.toggle();
    }

    async function selectTask(tasks: readonly AppTask[], taskId: string): Promise<void> {
        const selected = tasks.find((task) => task.id === taskId);
        if (selected === undefined || selected.status === 'done') return;

        await deps.closeMenu();
        await stopwatchState.startTask(selected);
    }

    async function addAndTrackTask(title: string): Promise<void> {
        const trimmed = title.trim();
        if (trimmed === '') return;

        const newTask: AppTask = {
            id: crypto.randomUUID(),
            title: trimmed,
            status: 'todo',
            priority: DEFAULT_TASK_PRIORITY,
        };

        const result = await store.mutate('Failed to create task', () => store.addTask(newTask));
        if (result.isOk()) {
            await deps.closeMenu();
            await stopwatchState.startTask(newTask);
        }
    }

    async function setCustomLabel(text: string): Promise<void> {
        const trimmed = text.trim();
        await deps.closeMenu();

        if (trimmed === '') {
            await stopwatchState.setLabel(null, 'unset');
            return;
        }

        await stopwatchState.setLabel(trimmed, 'custom');
        if (stopwatchState.phase === 'paused' || stopwatchState.phase === 'stopped') {
            await stopwatchState.toggle();
        }
    }

    async function stopWorking(activeTask: AppTask | undefined): Promise<void> {
        if (activeTask === undefined) return;

        await deps.closeMenu();
        await stopClock();
        await stopwatchState.clearTask();
    }

    async function toggleDone(task: AppTask): Promise<void> {
        const next: AppTaskStatus = task.status === 'done' ? 'todo' : 'done';
        await setTaskStatus(task, next);
        if (next === 'done' && stopwatchState.taskId === task.id) {
            await stopClock();
            await stopwatchState.clearTask();
        }
    }

    function executeKeyCommand(cmd: MinitrackerKeyCommand): void {
        switch (cmd.kind) {
            case 'toggleBreak':
                void stopwatchState.toggleBreak(deps.getBreakDivisor());
                break;
            case 'closeMenu':
                void deps.closeMenu();
                break;
            case 'toggleDimmed':
                deps.toggleDimmed();
                break;
            case 'restoreMainWindow':
                void safeInvoke(commands.windowRestoreMain());
                break;
            case 'togglePause':
                void stopwatchState.togglePause(deps.getPauseMinutes());
                break;
            case 'adjustPause':
                void stopwatchState.adjustPause(cmd.delta);
                break;
            case 'toggleTransport':
                void stopwatchState.toggle();
                break;
        }
    }

    return {
        selectTask,
        addAndTrackTask,
        setCustomLabel,
        stopWorking,
        toggleDone,
        executeKeyCommand,
    };
}
