import { persistedKeys } from './persisted.svelte';
import { safeInvoke } from './safeInvoke.svelte';
import { minutesSinceMidnight, ymd } from './time';

/** Local time (minutes since midnight) the daily startup screen may first open. */
export const DAILY_STARTUP_OPEN_MINUTES = 4 * 60 + 30;

/** True when the app has not yet been opened on `today` (local date). */
export function isFirstOpenToday(lastOpened: string | null | undefined, today: string): boolean {
    return lastOpened !== today;
}

/**
 * Claims the first launch of the day, recording today's local date so a later
 * launch in the same day skips the startup screen. Launches before the daily
 * cutoff (4:30 AM) belong to the previous night and never claim the day, so the
 * screen waits until the morning. Backend failures leave the caller on its
 * default route rather than blocking startup.
 */
export async function claimFirstOpenOfDay(now: Date = new Date()): Promise<boolean> {
    if (minutesSinceMidnight(now) < DAILY_STARTUP_OPEN_MINUTES) return false;

    const today = ymd(now);
    const stored = await safeInvoke<string | null>('get_ui_state', {
        key: persistedKeys.appLastOpened,
    });
    if (stored.isErr()) return false;
    if (!isFirstOpenToday(stored.value, today)) return false;

    const saved = await safeInvoke('set_ui_state', {
        key: persistedKeys.appLastOpened,
        value: today,
    });
    return saved.isOk();
}
