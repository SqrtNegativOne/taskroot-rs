<script lang="ts">
    import { splitTime, type StopwatchState } from './engine.svelte';
    import { useNow } from '../../../lib/useNow.svelte';

    const NOW_INTERVAL_MS = 250;

    let { engine, onToggle }: {
        engine: StopwatchState;
        onToggle: () => void;
    } = $props();

    const now = useNow(NOW_INTERVAL_MS);

    let displayData = $derived.by(() => {
        void now.ms;
        if (engine.isPaused) {
            const { m, s } = splitTime(engine.pauseRemainingMs);
            return {
                primaryText: `PAUSED ${m}:${s}`,
                showPlayIcon: false
            };
        }
        if (engine.isBreak) {
            const value = engine.isCountdown ? engine.breakRemainingMs : engine.breakElapsedMs;
            const { m, s } = splitTime(value);
            return {
                primaryText: `${m}:${s}`,
                showPlayIcon: false
            };
        }
        if (engine.isStopped) {
            return {
                primaryText: '00:00',
                showPlayIcon: false
            };
        }
        const { m } = splitTime(engine.workMs);
        return {
            primaryText: m,
            showPlayIcon: engine.isPristine
        };
    });
</script>

<button
    type="button"
    aria-label="Toggle stopwatch"
    class="stopwatch-display"
    class:is-running={engine.phase === 'running'}
    class:is-pristine={engine.isPristine}
    class:is-paused={engine.isPaused}
    class:is-breaking={engine.phase === 'breaking'}
    class:is-stopped={engine.phase === 'stopped'}
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
