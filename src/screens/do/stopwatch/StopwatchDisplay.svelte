<script lang="ts">
    import { splitTime, type StopwatchState } from './engine.svelte';

    let { engine, onToggle } = $props<{
        engine: StopwatchState,
        onToggle: () => void
    }>();

    let tick = $state(0);

    $effect(() => {
        let raf: number;
        const loop = () => {
            tick++;
            raf = requestAnimationFrame(loop);
        };
        if (engine.running) {
            raf = requestAnimationFrame(loop);
        }
        return () => {
            if (raf) cancelAnimationFrame(raf);
        };
    });

    let displayData = $derived.by(() => {
        // Depend on tick to recalculate on animation frame
        tick;
        const { m } = splitTime(engine.currentMs);
        return {
            primaryText: m,
            showPlayIcon: engine.isPristine
        };
    });
</script>

<button
    type="button"
    aria-label="Toggle stopwatch"
    class="stopwatch-display {engine.running ? 'is-running' : ''} {engine.isPristine ? 'is-pristine' : ''}"
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
