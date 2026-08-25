<script lang="ts">
    import { splitTime, type StopwatchState } from './engine.svelte';
    import { useNow } from '../../../lib/useNow.svelte';
    import { store } from '../../../lib/store.svelte';

    const NOW_INTERVAL_MS = 250;

    let { engine, onToggle } = $props<{
        engine: StopwatchState,
        onToggle: () => void
    }>();

    const now = useNow(NOW_INTERVAL_MS);

    let displayData = $derived.by(() => {
        void now.ms;
        const { m } = splitTime(engine.currentMs);
        const clockStyle = store.settings?.clock_style;
        const showPlayIcon = clockStyle === 'guzey' ? false : engine.isPristine;
        return {
            primaryText: m,
            showPlayIcon
        };
    });
</script>

<button
    type="button"
    aria-label="Toggle stopwatch"
    class="stopwatch-display"
    class:is-running={engine.running}
    class:is-pristine={engine.isPristine}
    onclick={onToggle}
    title="Click to start/stop"
>
    <span class="sw-digits sw-m row">
        {#if displayData.showPlayIcon}
            <svg
                width="0.8em"
                height="0.8em"
                viewBox="0 0 24 24"
                fill="currentColor"
                xmlns="http://www.w3.org/2000/svg"
                class="sw-play-icon"
            >
                <path d="M8 5v14l11-7z" />
            </svg>
        {:else}
            <span class="sw-primary-text">{displayData.primaryText}</span>
        {/if}
    </span>
</button>
