import { SvelteDate } from 'svelte/reactivity';

const DEFAULT_NOW_INTERVAL_MS = 1000;

export function useNow(intervalMs: number = DEFAULT_NOW_INTERVAL_MS) {
    const now = new SvelteDate();

    $effect(() => {
        now.setTime(Date.now());
        const intervalId = setInterval(() => {
            now.setTime(Date.now());
        }, intervalMs);
        return () => clearInterval(intervalId);
    });

    return {
        get value(): Date {
            return now;
        },
        get ms(): number {
            return now.getTime();
        },
    };
}
