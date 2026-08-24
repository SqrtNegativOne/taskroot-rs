import { listen } from '@tauri-apps/api/event';
import { safeInvoke } from '../../../lib/safeInvoke.svelte';
import { STOPWATCH_UPDATED } from '../../../lib/events';
import type { StopwatchState as StopwatchSnapshot } from '../../../lib/domain';

export class StopwatchState {
    elapsed = $state(0);
    runningSince = $state<number | undefined>(undefined);
    isBreak = $state(false);
    breakAllowedMs = $state(5 * 60 * 1000);
    breakStartedAt = $state<number | undefined>(undefined);
    breakSoundPlayed = $state(false);

    private unlisten: (() => void) | undefined;
    private connection: Promise<void> | undefined;

    constructor() {
        void this.init();
    }

    async init(): Promise<void> {
        const result = await safeInvoke<StopwatchSnapshot>('get_stopwatch_state');
        if (result.isOk()) this.updateFromPayload(result.value);

        await this.connect();
    }

    dispose(): void {
        this.unlisten?.();
        this.unlisten = undefined;
        this.connection = undefined;
    }

    private connect(): Promise<void> {
        this.connection ??= listen<StopwatchSnapshot>(STOPWATCH_UPDATED, (event) => {
            this.updateFromPayload(event.payload);
        }).then((unlisten) => {
            this.unlisten = unlisten;
        });
        return this.connection;
    }

    updateFromPayload(payload: StopwatchSnapshot): void {
        this.elapsed = payload.elapsed;
        this.runningSince = payload.runningSince ?? undefined;
        this.isBreak = payload.isBreak;
        this.breakAllowedMs = payload.breakAllowedMs;
        this.breakStartedAt = payload.breakStartedAt ?? undefined;
        this.breakSoundPlayed = payload.breakSoundPlayed;
    }

    get running() {
        return this.runningSince !== undefined;
    }

    get currentMs() {
        return this.elapsed + (this.running && !this.isBreak ? Date.now() - (this.runningSince ?? 0) : 0);
    }

    get isPristine() {
        return this.currentMs === 0 && !this.running && !this.isBreak;
    }

    async toggle(): Promise<void> {
        const result = await safeInvoke<StopwatchSnapshot>('toggle_stopwatch');
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
    return {
        m: totalMin.toString().padStart(2, '0'),
    };
}
