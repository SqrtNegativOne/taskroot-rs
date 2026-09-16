<script lang="ts">
    import { onMount } from 'svelte';
    import { getCurrentWindow, currentMonitor, LogicalPosition } from '@tauri-apps/api/window';
    import { store } from '../../lib/store.svelte';
    import { safeInvoke } from '../../lib/safeInvoke.svelte';
    import { persistState, persistedKeys } from '../../lib/persisted.svelte';
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

    import { useTauriQuery } from '../../lib/safeInvoke.svelte';
    import type { AppTask } from '../../lib/domain';

    let isDimmed = $state(false);
    let isClickDimmed = $state(false);
    let baseAlpha = $state(0.8);

    const pauseMinutes = $derived(store.settings?.pause_minutes ?? 10);

    // Seed the wheel-adjusted opacity from the persisted setting once settings
    // load (and re-seed if another window changes it).
    $effect(() => {
        if (store.settings) baseAlpha = store.settings.tracker_opacity / 100;
    });

    const effectiveOpacity = $derived.by(() => {
        if (isDimmed) return 0.2;
        if (isClickDimmed) return Math.max(0.2, baseAlpha - 0.15);
        return baseAlpha;
    });

    interface StoredPosition { x: number; y: number }

    function isStoredPosition(value: unknown): value is StoredPosition {
        if (typeof value !== 'object' || value === null) return false;
        const candidate = value as { x?: unknown; y?: unknown };
        return typeof candidate.x === 'number' && typeof candidate.y === 'number';
    }

    let savedPosition = $state<StoredPosition>({ x: 0, y: 0 });

    persistState<StoredPosition>(
        persistedKeys.minitrackerPosition,
        () => savedPosition,
        (position) => {
            savedPosition = position;
            void getCurrentWindow().setPosition(new LogicalPosition(position.x, position.y));
        },
        { isValid: isStoredPosition },
    );

    function adjustOpacity(delta: number): void {
        const next = Math.min(1, Math.max(0.2, baseAlpha + delta));
        const percentage = Math.round(next * 100);
        if (percentage === Math.round(baseAlpha * 100)) return;
        baseAlpha = percentage / 100;
        if (store.settings) store.settings.tracker_opacity = percentage;
        void safeInvoke('update_setting', { key: 'tracker_opacity', value: percentage });
    }

    function handleWheel(event: WheelEvent): void {
        event.preventDefault();
        adjustOpacity(event.deltaY < 0 ? 0.05 : -0.05);
    }

    let tasksQuery = useTauriQuery<AppTask[]>('query_tasks');
    let tasks = $derived(tasksQuery.data ?? []);

    $effect(() => {
        void tasksQuery.execute({ filters: [], sort: [], query: "" });
    });

    let activeTask = $derived.by(() => {
        return tasks.find(t => t.status === 'doing');
    });

    const now = useNow();

    let currentMs = $derived.by(() => {
        void now.ms;
        return stopwatchState.isPaused ? stopwatchState.pauseRemainingMs : stopwatchState.currentMs;
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
        void getCurrentWindow().setAlwaysOnTop(true);
        const handleKeyDown = (e: KeyboardEvent) => {
            const shortcut = KEYBOARD_SHORTCUTS.find((s) => matchesShortcut(e, s));
            if (shortcut) {
                e.preventDefault();
                void runShortcut(shortcut.action);
                return;
            }
            if (e.key === 'p' || e.key === 'P') {
                e.preventDefault();
                void stopwatchState.togglePause(pauseMinutes);
                return;
            }
            if ((e.key === 'ArrowUp' || e.key === 'ArrowRight') && stopwatchState.isPaused) {
                e.preventDefault();
                void stopwatchState.adjustPause(1);
                return;
            }
            if ((e.key === 'ArrowDown' || e.key === 'ArrowLeft') && stopwatchState.isPaused) {
                e.preventDefault();
                void stopwatchState.adjustPause(-1);
            }
        };

        window.addEventListener('keydown', handleKeyDown);

        return () => {
            window.removeEventListener('keydown', handleKeyDown);
        };
    });

    let isDragging = false;
    let readyToDrag = false;
    let dragStartX = 0;
    let dragStartY = 0;
    let initialWinX = 0;
    let initialWinY = 0;
    
    let monitorRect: { left: number, right: number, top: number, bottom: number } | null = null;
    let winSize: { width: number, height: number } | null = null;

    let pendingX = 0;
    let pendingY = 0;
    let isSettingPosition = false;
    let wantsToSetPosition = false;

    const SNAP_THRESHOLD = 24;

    const handlePointerDown = async (e: PointerEvent) => {
        if (e.button !== 0) return;
        const target = e.currentTarget as HTMLElement;
        target.setPointerCapture(e.pointerId);

        isDragging = true;
        readyToDrag = false;
        isClickDimmed = true;
        dragStartX = e.screenX;
        dragStartY = e.screenY;

        const appWindow = getCurrentWindow();
        
        const [pos, factor, size, monitor] = await Promise.all([
            appWindow.outerPosition(),
            appWindow.scaleFactor(),
            appWindow.outerSize(),
            currentMonitor()
        ]);
        
        if (!isDragging) return;

        initialWinX = pos.x / factor;
        initialWinY = pos.y / factor;
        winSize = { width: size.width / factor, height: size.height / factor };
        
        if (monitor) {
            const mFactor = monitor.scaleFactor;
            monitorRect = {
                left: monitor.workArea.position.x / mFactor,
                top: monitor.workArea.position.y / mFactor,
                right: (monitor.workArea.position.x + monitor.workArea.size.width) / mFactor,
                bottom: (monitor.workArea.position.y + monitor.workArea.size.height) / mFactor,
            };
        } else {
            monitorRect = null;
        }

        readyToDrag = true;
        pendingX = initialWinX;
        pendingY = initialWinY;
    };

    async function updatePosition() {
        if (isSettingPosition || !wantsToSetPosition) return;
        isSettingPosition = true;
        wantsToSetPosition = false;
        try {
            await getCurrentWindow().setPosition(new LogicalPosition(pendingX, pendingY));
        } finally {
            isSettingPosition = false;
            if (wantsToSetPosition) {
                requestAnimationFrame(() => { void updatePosition(); });
            }
        }
    }

    const handlePointerMove = (e: PointerEvent) => {
        if (!isDragging || !readyToDrag) return;
        
        let newX = initialWinX + (e.screenX - dragStartX);
        let newY = initialWinY + (e.screenY - dragStartY);

        if (monitorRect && winSize) {
            if (Math.abs(newX - monitorRect.left) < SNAP_THRESHOLD) {
                newX = monitorRect.left;
            } else if (Math.abs((newX + winSize.width) - monitorRect.right) < SNAP_THRESHOLD) {
                newX = monitorRect.right - winSize.width;
            }

            if (Math.abs(newY - monitorRect.top) < SNAP_THRESHOLD) {
                newY = monitorRect.top;
            } else if (Math.abs((newY + winSize.height) - monitorRect.bottom) < SNAP_THRESHOLD) {
                newY = monitorRect.bottom - winSize.height;
            }
        }

        pendingX = newX;
        pendingY = newY;
        wantsToSetPosition = true;
        void updatePosition();
    };

    const handlePointerUp = (e: PointerEvent) => {
        if (!isDragging) return;
        const wasDraggable = readyToDrag;
        isDragging = false;
        readyToDrag = false;
        isClickDimmed = false;
        const target = e.currentTarget as HTMLElement;
        target.releasePointerCapture(e.pointerId);
        if (wasDraggable) {
            savedPosition = { x: pendingX, y: pendingY };
        }
    };

    const handleDoubleClick = async () => {
        await safeInvoke('quit_app');
    };
</script>

<div
    role="region"
    aria-label="Mini tracker"
    class="minitracker-container"
    style="opacity: {effectiveOpacity}"
    ondblclick={handleDoubleClick}
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    onpointercancel={handlePointerUp}
    onwheel={handleWheel}
    title="Double-click to quit. Scroll to change opacity."
>
    <div class="clock" style="color: {textColor}">
        <div class="time">{timeParts.m}:{timeParts.s}</div>
        <div class="byline">
            {#if stopwatchState.isPaused}
                paused
            {:else if store.settings?.clock_style === 'guzey'}
                {#if stopwatchState.activePhase === 'long break'}
                    long break
                {:else if stopwatchState.activePhase === 'break'}
                    break
                {:else}
                    {activeTask ? activeTask.title : 'work'}
                {/if}
            {:else if stopwatchState.activePhase === 'break'}
                {#if stopwatchState.isCountdown}
                    left in break
                {:else}
                    in break
                {/if}
            {:else}
                {#if stopwatchState.isCountdown}
                    left {activeTask ? `for ${activeTask.title}` : 'for Work session'}
                {:else}
                    elapsed {activeTask ? `for ${activeTask.title}` : 'for Work session'}
                {/if}
            {/if}
        </div>
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
        overflow: hidden;
        container-type: size;
        border-radius: 8px; /* Optional: rounding corners */
        box-shadow: inset 0 0 0 2px rgba(255, 255, 255, 0.3);
        border: 1px solid rgba(255, 255, 255, 0.15);
    }

    .clock {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        width: 100%;
        gap: 4px;
    }

    .time {
        font-size: min(55cqh, 28cqw);
        font-family: monospace;
        line-height: 1;
        font-weight: normal;
    }

    .byline {
        font-size: min(20cqh, 10cqw, 15px);
        opacity: 0.8;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        max-width: 95%;
    }
</style>
