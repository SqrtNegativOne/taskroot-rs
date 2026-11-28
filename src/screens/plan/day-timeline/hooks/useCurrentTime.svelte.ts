import { minutesSinceMidnight } from '../../../../lib/time';
import { useNow } from '../../../../lib/useNow.svelte';

export function useCurrentTime() {
    const now = useNow(60_000);

    return {
        get value() {
            return minutesSinceMidnight(now.value);
        },
    };
}
