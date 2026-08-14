import { store } from '../../../lib/store.svelte';
import type { AppTask } from '../../../lib/domain';

// A simple local stopwatch store since we don't have the full store yet
export class StopwatchState {
    elapsed = $state(0);
    runningSince = $state<number | undefined>(undefined);
    isBreak = $state(false);
    breakAllowedMs = $state(5 * 60 * 1000);
    breakStartedAt = $state<number | undefined>(undefined);
    breakSoundPlayed = $state(false);

    get running() {
        return this.runningSince !== undefined;
    }

    get currentMs() {
        return this.elapsed + (this.running && !this.isBreak ? Date.now() - (this.runningSince || 0) : 0);
    }

    get isPristine() {
        return this.currentMs === 0 && !this.running && !this.isBreak;
    }

    toggle() {
        if (this.runningSince) {
            // Stop
            this.elapsed += Date.now() - this.runningSince;
            this.runningSince = undefined;
            // TODO: Log session
        } else {
            // Start
            this.runningSince = Date.now();
        }
    }

    reset() {
        this.elapsed = 0;
        this.runningSince = undefined;
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
