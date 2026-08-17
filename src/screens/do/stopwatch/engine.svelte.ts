import { safeInvoke } from '../../../lib/safeInvoke.svelte';
import { listen } from '@tauri-apps/api/event';

export class StopwatchState {
    elapsed = $state(0);
    runningSince = $state<number | undefined>(undefined);
    isBreak = $state(false);
    breakAllowedMs = $state(5 * 60 * 1000);
    breakStartedAt = $state<number | undefined>(undefined);
    breakSoundPlayed = $state(false);

    constructor() {
        this.init();
    }

    async init() {
        const result = await safeInvoke<any>('get_stopwatch_state');
        if (result.isOk()) {
            this.updateFromPayload(result.value);
        }

        listen('stopwatch-updated', (event) => {
            this.updateFromPayload(event.payload as any);
        });
    }

    updateFromPayload(payload: any) {
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
        return this.elapsed + (this.running && !this.isBreak ? Date.now() - (this.runningSince || 0) : 0);
    }

    get isPristine() {
        return this.currentMs === 0 && !this.running && !this.isBreak;
    }

    async toggle() {
        const result = await safeInvoke<any>('toggle_stopwatch');
        if (result.isOk()) {
            this.updateFromPayload(result.value);
        }
    }

    async reset() {
        const result = await safeInvoke<any>('reset_stopwatch');
        if (result.isOk()) {
            this.updateFromPayload(result.value);
        }
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
