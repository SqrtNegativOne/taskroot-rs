import { listen } from '@tauri-apps/api/event';
import { safeInvoke } from '../../../lib/safeInvoke.svelte';
import { STOPWATCH_UPDATED } from '../../../lib/events';
import type { StopwatchState as StopwatchSnapshot } from '../../../lib/domain';
import { store } from '../../../lib/store.svelte';

export class StopwatchState {
    elapsed = $state(0);
    runningSince = $state<number | undefined>(undefined);
    isBreak = $state(false);
    breakElapsed = $state(0);
    breakRunningSince = $state<number | undefined>(undefined);

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
        this.breakElapsed = payload.breakElapsed;
        this.breakRunningSince = payload.breakRunningSince ?? undefined;
    }

    get running() {
        return this.runningSince !== undefined;
    }

    get isCountdown() {
        if (!store.loaded || !store.settings) return false;
        const style = store.settings.clock_style;
        if (style === 'guzey') return true;
        if (style === 'flowtime' && this.isBreak) return true;
        return false;
    }

    get activePhase(): 'work' | 'break' {
        if (!store.loaded || !store.settings) return this.isBreak ? 'break' : 'work';
        const style = store.settings.clock_style;
        
        if (style === 'guzey') {
            // eslint-disable-next-line svelte/prefer-svelte-reactivity
            const date = new Date();
            const hour = date.getHours();
            const min = date.getMinutes();
            if (hour % 3 === 0 && min < 35) return 'break';
            if (min >= 0 && min < 5) return 'break';
            if (min >= 30 && min < 35) return 'break';
            return 'work';
        }
        return this.isBreak ? 'break' : 'work';
    }

    get currentMs() {
        if (!store.loaded || !store.settings) return 0;
        const style = store.settings.clock_style;
        
        const nowMs = Date.now();
        if (style === 'guzey') {
            // eslint-disable-next-line svelte/prefer-svelte-reactivity
            const date = new Date(nowMs);
            const hour = date.getHours();
            const min = date.getMinutes();
            const sec = date.getSeconds();
            const ms = date.getMilliseconds();
            
            let targetMin = 60;
            if (hour % 3 === 0 && min < 35) targetMin = 35;
            else if (min < 5) targetMin = 5;
            else if (min < 30) targetMin = 30;
            else if (min < 35) targetMin = 35;
            
            const msLeft = (targetMin * 60 * 1000) - ((min * 60 * 1000) + (sec * 1000) + ms);
            return Math.max(0, msLeft);
        }
        
        if (this.isBreak) {
            const breakTaken = this.breakElapsed + (this.breakRunningSince !== undefined ? nowMs - this.breakRunningSince : 0);
            if (style === 'flowtime') {
                const totalWork = this.elapsed; // For simplicity, only completed work chunks count. If you want ongoing work to count, it requires more complex math.
                const breakDivisor = store.settings.flowtime_break_divisor || 5;
                const breakEarned = totalWork / breakDivisor;
                return Math.max(0, breakEarned - breakTaken); 
            }
            return breakTaken;
        }
        
        return this.elapsed + (this.running ? nowMs - (this.runningSince ?? 0) : 0);
    }

    get isPristine() {
        return this.currentMs === 0 && !this.running && !this.isBreak;
    }

    async toggle(): Promise<void> {
        const result = await safeInvoke<StopwatchSnapshot>('toggle_stopwatch');
        if (result.isOk()) this.updateFromPayload(result.value);
    }

    async toggleBreak(): Promise<void> {
        const result = await safeInvoke<StopwatchSnapshot>('toggle_break');
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

let lastPhase: 'work' | 'break' | undefined;

function playBeep() {
    import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
        if (getCurrentWindow().label !== 'main') return;
        try {
            const ctx = new window.AudioContext();
            const osc = ctx.createOscillator();
            const gain = ctx.createGain();
            osc.connect(gain);
            gain.connect(ctx.destination);
            osc.type = 'sine';
            osc.frequency.setValueAtTime(880, ctx.currentTime);
            gain.gain.setValueAtTime(0.1, ctx.currentTime);
            osc.start();
            osc.stop(ctx.currentTime + 0.15);
            
            setTimeout(() => {
                const osc2 = ctx.createOscillator();
                const gain2 = ctx.createGain();
                osc2.connect(gain2);
                gain2.connect(ctx.destination);
                osc2.type = 'sine';
                osc2.frequency.setValueAtTime(1046.5, ctx.currentTime);
                gain2.gain.setValueAtTime(0.1, ctx.currentTime);
                osc2.start();
                osc2.stop(ctx.currentTime + 0.15);
            }, 200);
        } catch (e) {
            console.error('Failed to play beep', e);
        }
    }).catch(() => { /* ignore */ });
}

if (typeof window !== 'undefined') {
    setInterval(() => {
        const currentPhase = stopwatchState.activePhase;
        if (lastPhase !== undefined && lastPhase !== currentPhase) {
            playBeep();
        }
        lastPhase = currentPhase;
    }, 1000);
}
