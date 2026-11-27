import { getCurrentWindow } from '@tauri-apps/api/window';
import type { StopwatchPhase } from '../../../lib/domain';
import { commands, events, safeInvoke } from '../../../lib/safeInvoke.svelte';

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
    const result = await safeInvoke(commands.showScreenEffect());
    if (result.isErr()) return;
    void events.screenEffect.emit();
}

export function playErrorSound(): void {
    playSound('windows-xp-error.mp3');
}

/** Called only when entering or leaving a break; stopped means the break ended. */
export function notifyBreakTransition(phase: StopwatchPhase): void {
    if (phase === 'stopped') {
        playSound('wine-glass-alarm.ogg');
        return;
    }
    playSound(phase === 'breaking' ? 'work_to_break.wav' : 'break_to_work.wav');
    void triggerScreenEffect();
}
