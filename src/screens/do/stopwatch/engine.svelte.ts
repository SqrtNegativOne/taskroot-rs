import { listen } from '@tauri-apps/api/event';
import { safeInvoke } from '../../../lib/safeInvoke.svelte';
import { STOPWATCH_UPDATED } from '../../../lib/ipcEvents';
import type {
    StopwatchLabelSource,
    StopwatchPhase,
    StopwatchState as StopwatchSnapshot,
} from '../../../lib/domain';
import { store } from '../../../lib/store.svelte';
import { useNow } from '../../../lib/useNow.svelte';
import { notifyBreakTransition, playErrorSound } from './feedback';

let now: { get ms(): number };
$effect.root(() => {
    now = useNow();
});

export class StopwatchState {
    phase = $state<StopwatchPhase>('paused');
    workElapsed = $state(0);
    workRunningSince = $state<number | undefined>(undefined);
    breakDuration = $state(0);
    breakStartedAt = $state<number | undefined>(undefined);
    pausedUntil = $state<number | undefined>(undefined);
    taskId = $state<string | undefined>(undefined);
    label = $state<string | undefined>(undefined);
    labelSource = $state<StopwatchLabelSource>('unset');

    private unlisten: (() => void) | undefined;
    private connection: Promise<void> | undefined;
    private disposed = false;

    constructor() {
        void this.init();
    }

    async init(): Promise<void> {
        const result = await safeInvoke<StopwatchSnapshot | undefined>('get_stopwatch_state');
        if (this.disposed) return;
        if (result.isOk() && result.value !== undefined) this.updateFromPayload(result.value);

        await this.connect();
    }

    dispose(): void {
        this.disposed = true;
        this.unlisten?.();
        this.unlisten = undefined;
        this.connection = undefined;
    }

    private connect(): Promise<void> {
        if (this.disposed) return Promise.resolve();
        this.connection ??= listen<StopwatchSnapshot>(STOPWATCH_UPDATED, (event) => {
            this.updateFromPayload(event.payload);
        }).then((unlisten) => {
            if (this.disposed) {
                unlisten();
                return;
            }
            this.unlisten = unlisten;
        });
        return this.connection;
    }

    updateFromPayload(payload: StopwatchSnapshot): void {
        this.phase = payload.phase;
        this.workElapsed = payload.workElapsed;
        this.workRunningSince = payload.workRunningSince ?? undefined;
        this.breakDuration = payload.breakDuration;
        this.breakStartedAt = payload.breakStartedAt ?? undefined;
        this.pausedUntil = payload.pausedUntil ?? undefined;
        this.taskId = payload.taskId ?? undefined;
        this.label = payload.label ?? undefined;
        this.labelSource = payload.labelSource;
    }

    get running() {
        return this.phase === 'running';
    }

    get isBreak() {
        return this.phase === 'breaking';
    }

    get isStopped() {
        return this.phase === 'stopped';
    }

    get isCountdown() {
        return this.isBreak && store.settings?.clock_style === 'flowtime';
    }

    get isPaused(): boolean {
        return this.pausedUntil !== undefined && this.pausedUntil > now.ms;
    }

    get pauseRemainingMs(): number {
        return this.pausedUntil === undefined ? 0 : Math.max(0, this.pausedUntil - now.ms);
    }

    get workMs(): number {
        if (this.phase !== 'running') return this.workElapsed;
        if (this.workRunningSince !== undefined) {
            return this.workElapsed + Math.max(0, now.ms - this.workRunningSince);
        }
        if (this.pausedUntil !== undefined && now.ms >= this.pausedUntil) {
            return this.workElapsed + (now.ms - this.pausedUntil);
        }
        return this.workElapsed;
    }

    get breakElapsedMs(): number {
        if (!this.isBreak || this.breakStartedAt === undefined) return 0;
        return Math.max(0, now.ms - this.breakStartedAt);
    }

    get breakRemainingMs(): number {
        return Math.max(0, this.breakDuration - this.breakElapsedMs);
    }

    get currentMs(): number {
        if (this.isPaused) return this.pauseRemainingMs;
        if (this.isBreak) return this.isCountdown ? this.breakRemainingMs : this.breakElapsedMs;
        if (this.isStopped) return 0;
        return this.workMs;
    }

    get isPristine() {
        return this.phase === 'paused' && this.workMs === 0 && !this.isPaused;
    }

    async toggle(): Promise<void> {
        const result = await safeInvoke<StopwatchSnapshot>('toggle_stopwatch');
        if (result.isErr()) {
            playErrorSound();
            return;
        }
        this.updateFromPayload(result.value);
    }

    async toggleBreak(breakDivisor: number): Promise<void> {
        const result = await safeInvoke<StopwatchSnapshot>('toggle_break', { breakDivisor });
        if (result.isErr()) {
            playErrorSound();
            return;
        }
        this.updateFromPayload(result.value);
    }

    async completeBreak(): Promise<void> {
        const result = await safeInvoke<StopwatchSnapshot>('complete_break');
        if (result.isOk()) this.updateFromPayload(result.value);
    }

    async setLabel(label: string | null, source: StopwatchLabelSource): Promise<void> {
        const result = await safeInvoke<StopwatchSnapshot>('set_stopwatch_label', { label, source });
        if (result.isOk()) this.updateFromPayload(result.value);
    }

    async setTask(taskId: string | null, label: string | null): Promise<boolean> {
        const result = await safeInvoke<StopwatchSnapshot>('set_stopwatch_task', { taskId, label });
        if (result.isErr()) return false;
        this.updateFromPayload(result.value);
        return true;
    }

    async startTask(task: { id: string; title: string }): Promise<void> {
        if (!(await this.setTask(task.id, task.title))) return;
        if (this.phase === 'paused' || this.phase === 'stopped') {
            await this.toggle();
        }
    }

    async clearTask(): Promise<void> {
        await this.setTask(null, null);
    }

    async togglePause(minutes: number): Promise<void> {
        const result = await safeInvoke<StopwatchSnapshot>('toggle_pause', { pauseMinutes: minutes });
        if (result.isOk()) this.updateFromPayload(result.value);
    }

    async adjustPause(deltaMinutes: number): Promise<void> {
        const result = await safeInvoke<StopwatchSnapshot>('adjust_pause', { deltaMinutes });
        if (result.isOk()) this.updateFromPayload(result.value);
    }

    async reset(): Promise<void> {
        const result = await safeInvoke<StopwatchSnapshot>('reset_stopwatch');
        if (result.isOk()) this.updateFromPayload(result.value);
    }
}

export const stopwatchState = new StopwatchState();

export function splitTime(ms: number) {
    const totalSec = Math.floor(ms / 1000);
    const totalMin = Math.floor(totalSec / 60);
    const remSec = totalSec % 60;
    return {
        m: totalMin.toString().padStart(2, '0'),
        s: remSec.toString().padStart(2, '0'),
    };
}

let lastInBreak: boolean | undefined;
let completingBreak = false;

if (typeof window !== 'undefined') {
    $effect.root(() => {
        $effect(() => {
            const inBreak = stopwatchState.phase === 'breaking';
            if (lastInBreak !== undefined && lastInBreak !== inBreak) {
                notifyBreakTransition(stopwatchState.phase);
            }
            lastInBreak = inBreak;
        });
    });

    $effect.root(() => {
        $effect(() => {
            void now.ms;
            if (completingBreak) return;
            if (stopwatchState.phase !== 'breaking') return;
            if (store.settings?.clock_style !== 'flowtime') return;
            if (stopwatchState.breakRemainingMs > 0) return;

            completingBreak = true;
            void stopwatchState.completeBreak().finally(() => {
                completingBreak = false;
            });
        });
    });
}
