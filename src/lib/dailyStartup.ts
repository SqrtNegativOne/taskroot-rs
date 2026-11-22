import { persistedKeys } from './persisted.svelte';
import { safeInvoke } from './safeInvoke.svelte';
import { ymd } from './time';

/** True when the app has not yet been opened on `today` (local date). */
export function isFirstOpenToday(lastOpened: string | null | undefined, today: string): boolean {
    return lastOpened !== today;
}

/**
 * Claims the first launch of the day, recording today's local date so a later
 * launch in the same day skips the startup screen. Backend failures leave the
 * caller on its default route rather than blocking startup.
 */
export async function claimFirstOpenOfDay(now: Date = new Date()): Promise<boolean> {
    const today = ymd(now);
    const stored = await safeInvoke<string | null>('get_ui_state', {
        key: persistedKeys.appLastOpened,
    });
    if (stored.isErr()) return false;
    if (!isFirstOpenToday(stored.value, today)) return false;

    await safeInvoke('set_ui_state', { key: persistedKeys.appLastOpened, value: today });
    return true;
}
