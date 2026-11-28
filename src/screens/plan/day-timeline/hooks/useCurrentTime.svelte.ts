import { onMount } from 'svelte';
import { minutesSinceMidnight } from '../../../../lib/time';

export function useCurrentTime() {
    let nowMin = $state(minutesSinceMidnight(new Date()));

    function update() {
        nowMin = minutesSinceMidnight(new Date());
    }

    onMount(() => {
        update();
        const int = setInterval(update, 60000);
        return () => { clearInterval(int); };
    });

    return {
        get value() { return nowMin; }
    };
}
