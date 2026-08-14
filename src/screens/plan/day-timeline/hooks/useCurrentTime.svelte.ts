import { onMount } from 'svelte';
import { SvelteDate } from 'svelte/reactivity';
import { MINUTES_IN_HOUR } from '../types';

export function useCurrentTime() {
    let nowMin = $state(0);

    function update() {
        const now = new SvelteDate();
        nowMin = now.getHours() * MINUTES_IN_HOUR + now.getMinutes();
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
