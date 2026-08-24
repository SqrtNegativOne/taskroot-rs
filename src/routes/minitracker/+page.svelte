<script lang="ts">
    import { onMount } from 'svelte';
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import { store } from '../../lib/store.svelte';
    import { safeInvoke } from '../../lib/safeInvoke.svelte';
    import { useNow } from '../../lib/useNow.svelte';
    import { stopwatchState, splitTime } from '../../screens/do/stopwatch/engine.svelte';

    const BREAK_COLOR_HEX = '#3b82f6';
    const RUNNING_COLOR_HEX = '#eab308';
    const IDLE_COLOR_HEX = '#71717a';

    type ShortcutAction = 'toggleDimmed' | 'restoreMainWindow';

    interface KeyboardShortcut {
        readonly key: string;
        readonly ctrl?: boolean;
        readonly alt?: boolean;
        readonly action: ShortcutAction;
    }

    const KEYBOARD_SHORTCUTS: readonly KeyboardShortcut[] = [
        { key: 'h', action: 'toggleDimmed' },
        { key: 'r', ctrl: true, alt: true, action: 'restoreMainWindow' },
    ];

    function matchesShortcut(e: KeyboardEvent, shortcut: KeyboardShortcut): boolean {
        return (
            e.key === shortcut.key &&
            e.ctrlKey === !!shortcut.ctrl &&
            e.altKey === !!shortcut.alt &&
            !e.metaKey &&
            !e.shiftKey
        );
    }

    let isDimmed = $state(false);

    let activeTask = $derived.by(() => {
        return store.tasks.find(t => t.status === 'doing');
    });

    const now = useNow();

    let currentMs = $derived.by(() => {
        void now.ms;
        return stopwatchState.currentMs;
    });

    let timeParts = $derived(splitTime(currentMs));

    let textColor = $derived(
        stopwatchState.isBreak ? BREAK_COLOR_HEX : stopwatchState.running ? RUNNING_COLOR_HEX : IDLE_COLOR_HEX,
    );

    async function runShortcut(action: ShortcutAction) {
        if (action === 'toggleDimmed') {
            isDimmed = !isDimmed;
            return;
        }
        await safeInvoke('window_restore_main');
    }

    onMount(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            const shortcut = KEYBOARD_SHORTCUTS.find((s) => matchesShortcut(e, s));
            if (!shortcut) return;
            e.preventDefault();
            void runShortcut(shortcut.action);
        };

        window.addEventListener('keydown', handleKeyDown);

        return () => {
            window.removeEventListener('keydown', handleKeyDown);
        };
    });

    const handlePointerDown = async (e: PointerEvent) => {
        if (e.button !== 0) return;
        const appWindow = getCurrentWindow();
        await appWindow.startDragging();
    };

    const handleDoubleClick = async () => {
        await safeInvoke('window_restore_main');
    };
</script>

<div
    role="region"
    aria-label="Mini tracker"
    class="minitracker-container"
    class:is-dimmed={isDimmed}
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
        box-shadow: inset 0 0 0 2px rgba(255, 255, 255, 0.3);
        border: 1px solid rgba(255, 255, 255, 0.15);
    }

    .minitracker-container.is-dimmed {
        opacity: 0.2;
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
