import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { safeInvoke } from '../../../lib/safeInvoke.svelte';
import { SCREEN_EFFECT, STOPWATCH_UPDATED } from '../../../lib/events';
import type { StopwatchState as StopwatchSnapshot } from '../../../lib/domain';
import { store } from '../../../lib/store.svelte';
import { useNow } from '../../../lib/useNow.svelte';
import { guzeyPhase, guzeyRemainingMs, type ClockPhase } from './phases';

export type { ClockPhase } from './phases';

let now: { get value(): Date; get ms(): number };
$effect.root(() => {
    now = useNow();
});

export class StopwatchState {
    elapsed = $state(0);
    runningSince = $state<number | undefined>(undefined);
    isBreak = $state(false);
    breakElapsed = $state(0);
    breakRunningSince = $state<number | undefined>(undefined);
    pausedUntil = $state<number | undefined>(undefined);

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
        this.pausedUntil = payload.pausedUntil ?? undefined;
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

    get isPaused(): boolean {
        return this.pausedUntil !== undefined && this.pausedUntil > now.ms;
    }

    get pauseRemainingMs(): number {
        return this.pausedUntil === undefined ? 0 : Math.max(0, this.pausedUntil - now.ms);
    }

    get activePhase(): ClockPhase {
        if (!store.loaded || !store.settings) return this.isBreak ? 'break' : 'work';
        const style = store.settings.clock_style;
        
        if (style === 'guzey') {
            const date = now.value;
            return guzeyPhase(date.getHours(), date.getMinutes(), store.settings.have_long_breaks);
        }
        return this.isBreak ? 'break' : 'work';
    }

    get currentMs() {
        if (!store.loaded || !store.settings) return 0;
        const style = store.settings.clock_style;
        
        const nowMs = now.ms;
        if (style === 'guzey') {
            const date = now.value;
            return guzeyRemainingMs(
                date.getHours(),
                date.getMinutes(),
                date.getSeconds(),
                date.getMilliseconds(),
                store.settings.have_long_breaks,
            );
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
        return this.currentMs === 0 && !this.running && !this.isBreak && !this.isPaused;
    }

    async toggle(): Promise<void> {
        const result = await safeInvoke<StopwatchSnapshot>('toggle_stopwatch');
        if (result.isOk()) this.updateFromPayload(result.value);
    }

    async toggleBreak(): Promise<void> {
        const result = await safeInvoke<StopwatchSnapshot>('toggle_break');
        if (result.isOk()) this.updateFromPayload(result.value);
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

const TRANSITION_SOUNDS: Record<ClockPhase, Partial<Record<ClockPhase, string>>> = {
    work: { break: 'work_to_break.wav', 'long break': 'work_to_long_break.wav' },
    break: { work: 'break_to_work.wav' },
    'long break': { work: 'long_break_to_work.wav' },
};

const soundCache = new Map<string, HTMLAudioElement>();

function isMainWindow(): boolean {
    return getCurrentWindow().label === 'main';
}

function playSound(file: string): void {
    if (!isMainWindow()) return;
    let audio = soundCache.get(file);
    if (audio === undefined) {
        audio = new Audio(`/sounds/${file}`);
        soundCache.set(file, audio);
    }
    audio.currentTime = 0;
    void audio.play().catch(() => { /* autoplay or decode failures are non-fatal */ });
}

async function triggerScreenEffect(): Promise<void> {
    if (!isMainWindow()) return;
    const result = await safeInvoke('show_screen_effect');
    if (result.isErr()) return;
    const { emit } = await import('@tauri-apps/api/event');
    void emit(SCREEN_EFFECT);
}

let lastPhase: ClockPhase | undefined;

if (typeof window !== 'undefined') {
    $effect.root(() => {
        $effect(() => {
            const currentPhase = stopwatchState.activePhase;
            if (lastPhase !== undefined && lastPhase !== currentPhase) {
                const file = TRANSITION_SOUNDS[lastPhase][currentPhase];
                if (file !== undefined) playSound(file);
                void triggerScreenEffect();
            }
            lastPhase = currentPhase;
        });
    });
}
