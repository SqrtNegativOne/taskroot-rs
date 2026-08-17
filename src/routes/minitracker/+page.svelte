<script lang="ts">
    import { onMount } from 'svelte';
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import { store } from '../../lib/store.svelte';
    import { stopwatchState, splitTime } from '../../screens/do/stopwatch/engine.svelte';

    let isDimmed = $state(false);
    let tick = $state(0);
    
    // We just need a simple clock display that shows the active task or work session
    let activeTask = $derived.by(() => {
        return store.tasks.find(t => t.status === 'doing');
    });

    let currentMs = $derived.by(() => {
        tick;
        return stopwatchState.currentMs;
    });
    
    let timeParts = $derived(splitTime(currentMs));

    let displayText = $derived.by(() => {
        let suffix = activeTask ? activeTask.title : "Work session";
        if (stopwatchState.isBreak) {
            suffix = "left in break";
        } else {
            suffix = `left for ${activeTask ? activeTask.title : 'Work session'}`;
        }
        return `${timeParts.m}m ${suffix}`;
    });

    let textColor = $derived(stopwatchState.isBreak ? '#3b82f6' : (stopwatchState.running ? '#eab308' : '#71717a'));

    onMount(() => {
        let raf: number;
        const loop = () => {
            tick++;
            raf = requestAnimationFrame(loop);
        };
        raf = requestAnimationFrame(loop);

        const appWindow = getCurrentWindow();

        const handleKeyDown = async (e: KeyboardEvent) => {
            if (e.key === 'h' && !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey) {
                isDimmed = !isDimmed;
            } else if (e.key === 'r' && e.ctrlKey && e.altKey) {
                e.preventDefault();
                // Restore main window
                const { invoke } = await import('@tauri-apps/api/core');
                invoke('window_restore_main');
            }
        };

        window.addEventListener('keydown', handleKeyDown);

        return () => {
            window.removeEventListener('keydown', handleKeyDown);
            cancelAnimationFrame(raf);
        };
    });

    const handlePointerDown = async (e: PointerEvent) => {
        if (e.button !== 0) return;
        const appWindow = getCurrentWindow();
        await appWindow.startDragging();
    };

    const handleDoubleClick = async () => {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('window_restore_main');
    };
</script>

<div
    role="application"
    class="minitracker-container"
    class:is-dimmed={isDimmed}
    class:show-border={true}
    ondblclick={handleDoubleClick}
    onpointerdown={handlePointerDown}
    title="Double-click to restore main window"
>
    <div class="clock" style="color: {textColor}">
        <span class="font-normal">{timeParts.m}m</span>
        {#if stopwatchState.isBreak}
            left in break
        {:else}
            left for {activeTask ? activeTask.title : 'Work session'}
        {/if}
    </div>
</div>

<style>
    :global(html), :global(body) {
        background: transparent !important;
        margin: 0;
        padding: 0;
        overflow: hidden;
    }

    .minitracker-container {
        width: 100vw;
        height: 100vh;
        background: rgb(24, 24, 24);
        transition: opacity 0.2s ease;
        color: var(--fg);
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 15px;
        user-select: none;
        cursor: move;
        padding: 16px;
        box-sizing: border-box;
        text-align: center;
        opacity: 0.8;
        overflow: hidden;
        container-type: size;
        border-radius: 8px; /* Optional: rounding corners */
    }

    .minitracker-container.is-dimmed {
        opacity: 0.2;
    }

    .minitracker-container.show-border {
        box-shadow: inset 0 0 0 2px rgba(255, 255, 255, 0.3);
        border: 1px solid rgba(255, 255, 255, 0.15);
    }

    .clock {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        max-width: 100%;
        /* dynamic sizing */
        font-size: min(70cqh, calc(130cqw / 20));
        font-family: monospace;
    }
</style>
