<script lang="ts">
    import { onMount } from 'svelte';
    import { getCurrentWindow, LogicalPosition } from '@tauri-apps/api/window';
    import { store } from '../../lib/store.svelte';
    import { safeInvoke, useAutoQuery, useTauriQuery } from '../../lib/safeInvoke.svelte';
    import { persistState, persistedKeys } from '../../lib/persisted.svelte';
    import { useNow } from '../../lib/useNow.svelte';
    import { reloadTaskListState, taskListQueryArgs } from '../../lib/taskList.svelte';
    import { stopwatchState, splitTime } from '../../screens/do/stopwatch/engine.svelte';
    import type { AppTask, AppTaskStatus } from '../../lib/domain';
    import TaskMenu from './TaskMenu.svelte';
    import {
        createMinitrackerController,
        isInteractiveTarget,
        isTaskMenuTarget,
        type StoredPosition,
    } from './controller.svelte';
    import { DEFAULT_BAR_HEIGHT } from './menuLayout';
    import './minitracker.css';

    const RUNNING_COLOR_HEX = '#ffffff';
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

    function isTypingTarget(target: EventTarget | null): boolean {
        return target instanceof HTMLElement && target.matches('input, textarea, [contenteditable]');
    }

    function isStoredPosition(value: unknown): value is StoredPosition {
        if (typeof value !== 'object' || value === null) return false;
        const candidate = value as { x?: unknown; y?: unknown };
        return typeof candidate.x === 'number' && typeof candidate.y === 'number';
    }

    let isDimmed = $state(false);
    let leftShiftDown = false;
    let rightShiftDown = false;
    let shiftBreakFired = false;
    let baseAlpha = $state(0.8);
    let menuBarHeight = $state(DEFAULT_BAR_HEIGHT);
    let barElement: HTMLDivElement | undefined = $state();
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

    const controller = createMinitrackerController({
        savePosition: (position) => {
            savedPosition = position;
        },
    });

    const pauseMinutes = $derived(store.settings?.pause_minutes ?? 10);

    $effect(() => {
        if (store.settings) baseAlpha = store.settings.tracker_opacity / 100;
    });

    const effectiveOpacity = $derived.by(() => {
        if (isDimmed) return 0.2;
        if (controller.isClickDimmed) return Math.max(0.2, baseAlpha - 0.15);
        return baseAlpha;
    });

    function adjustOpacity(delta: number): void {
        const next = Math.min(1, Math.max(0.2, baseAlpha + delta));
        const percentage = Math.round(next * 100);
        if (percentage === Math.round(baseAlpha * 100)) return;
        baseAlpha = percentage / 100;
        if (store.settings) store.settings.tracker_opacity = percentage;
        void safeInvoke('update_setting', { key: 'tracker_opacity', value: percentage });
    }

    function handleWheel(event: WheelEvent): void {
        if (isTaskMenuTarget(event.target)) return;
        event.preventDefault();
        adjustOpacity(event.deltaY < 0 ? 0.05 : -0.05);
    }

    let allTasksQuery = useTauriQuery<AppTask[]>('query_tasks');
    let allTasks = $derived(allTasksQuery.data ?? []);

    $effect(() => {
        void allTasksQuery.execute({ filters: [], sort: [], query: '' });
    });

    let listQuery = useAutoQuery<AppTask[]>('query_tasks', () => taskListQueryArgs(), { debounceMs: 150 });
    let tasks = $derived(listQuery.data ?? []);

    let activeTask = $derived(allTasks.find((task) => task.status === 'doing'));

    const now = useNow();

    let currentMs = $derived.by(() => {
        void now.ms;
        return stopwatchState.isPaused ? stopwatchState.pauseRemainingMs : stopwatchState.currentMs;
    });

    let timeParts = $derived(splitTime(currentMs));

    let textColor = $derived(
        stopwatchState.running && !stopwatchState.isPaused ? RUNNING_COLOR_HEX : IDLE_COLOR_HEX,
    );

    const barTitle = $derived(
        activeTask
            ? `Tracking ${activeTask.title}. Drag the clock to move, click the task name to switch. Double-click to quit.`
            : 'No active task. Drag the clock to move, click the task name to choose one. Double-click to quit.',
    );

    async function handleToggleMenu(): Promise<void> {
        if (controller.menuOpen) {
            await controller.closeMenu();
            return;
        }

        const rect = barElement?.getBoundingClientRect();
        if (rect === undefined) return;

        menuBarHeight = rect.height;
        await reloadTaskListState();
        controller.toggleMenu(tasks.length + (activeTask ? 1 : 0), {
            width: rect.width,
            height: rect.height,
        });
    }

    async function setTaskStatus(task: AppTask, status: AppTaskStatus): Promise<void> {
        if (task.status === status) return;
        await store.mutate('Failed to update task status', () => store.updateTask({ ...task, status }));
    }

    async function stopClock(): Promise<void> {
        if (stopwatchState.isPaused) await stopwatchState.togglePause(pauseMinutes);
        if (stopwatchState.running && !stopwatchState.isBreak) await stopwatchState.toggle();
    }

    async function selectTask(taskId: string): Promise<void> {
        const selected = tasks.find((task) => task.id === taskId);
        if (selected === undefined) return;

        await controller.closeMenu();
        for (const task of allTasks) {
            if (task.id === taskId) {
                await setTaskStatus(task, 'doing');
            } else if (task.status === 'doing') {
                await setTaskStatus(task, 'todo');
            }
        }

        if (!stopwatchState.running) {
            await stopwatchState.toggle();
        }
    }

    async function stopWorking(): Promise<void> {
        const task = activeTask;
        if (task === undefined) return;

        await controller.closeMenu();
        await setTaskStatus(task, 'todo');
        await stopClock();
    }

    async function toggleDone(task: AppTask): Promise<void> {
        const next: AppTaskStatus = task.status === 'done' ? 'todo' : 'done';
        await setTaskStatus(task, next);
        if (next === 'done' && task.status === 'doing') await stopClock();
    }

    async function runShortcut(action: ShortcutAction) {
        if (action === 'toggleDimmed') {
            isDimmed = !isDimmed;
            return;
        }
        await safeInvoke('window_restore_main');
    }

    function handleEscape(e: KeyboardEvent): boolean {
        if (e.key !== 'Escape' || !controller.menuOpen) return false;
        e.preventDefault();
        void controller.closeMenu();
        return true;
    }

    function handlePauseShortcut(e: KeyboardEvent): boolean {
        if (e.key === 'p' || e.key === 'P') {
            e.preventDefault();
            void stopwatchState.togglePause(pauseMinutes);
            return true;
        }
        if (!stopwatchState.isPaused) return false;

        if (e.key === 'ArrowUp' || e.key === 'ArrowRight') {
            e.preventDefault();
            void stopwatchState.adjustPause(1);
            return true;
        }
        if (e.key === 'ArrowDown' || e.key === 'ArrowLeft') {
            e.preventDefault();
            void stopwatchState.adjustPause(-1);
            return true;
        }
        return false;
    }

    function handleTransportShortcut(e: KeyboardEvent): boolean {
        if (e.code !== 'Space') return false;
        e.preventDefault();
        void stopwatchState.toggle();
        return true;
    }

    function handleShiftKeyDown(e: KeyboardEvent): boolean {
        if (e.code === 'ShiftLeft') {
            leftShiftDown = true;
        } else if (e.code === 'ShiftRight') {
            rightShiftDown = true;
        } else {
            return false;
        }

        if (leftShiftDown && rightShiftDown && !shiftBreakFired) {
            shiftBreakFired = true;
            e.preventDefault();
            void stopwatchState.toggleBreak();
        }
        return true;
    }

    function handleShiftKeyUp(e: KeyboardEvent): void {
        if (e.code === 'ShiftLeft') {
            leftShiftDown = false;
        } else if (e.code === 'ShiftRight') {
            rightShiftDown = false;
        } else {
            return;
        }
        shiftBreakFired = false;
    }

    function resetShiftState(): void {
        leftShiftDown = false;
        rightShiftDown = false;
        shiftBreakFired = false;
    }

    onMount(() => {
        void getCurrentWindow().setAlwaysOnTop(true);

        const handleKeyDown = (e: KeyboardEvent) => {
            if (handleShiftKeyDown(e)) return;

            if (isTypingTarget(e.target)) {
                handleEscape(e);
                return;
            }

            const shortcut = KEYBOARD_SHORTCUTS.find((s) => matchesShortcut(e, s));
            if (shortcut) {
                e.preventDefault();
                void runShortcut(shortcut.action);
                return;
            }
            if (handleEscape(e) || handlePauseShortcut(e) || handleTransportShortcut(e)) return;
        };
        const handleKeyUp = (e: KeyboardEvent) => {
            handleShiftKeyUp(e);
        };
        const handleBlur = () => {
            resetShiftState();
            void controller.closeMenu();
        };

        window.addEventListener('keydown', handleKeyDown);
        window.addEventListener('keyup', handleKeyUp);
        window.addEventListener('blur', handleBlur);

        return () => {
            window.removeEventListener('keydown', handleKeyDown);
            window.removeEventListener('keyup', handleKeyUp);
            window.removeEventListener('blur', handleBlur);
        };
    });

    const handleDoubleClick = async (e: MouseEvent) => {
        if (isInteractiveTarget(e.target)) return;
        await safeInvoke('window_restore_main');
    };
</script>

<div
    role="region"
    aria-label="Mini tracker"
    class="minitracker-container"
    class:menu-up={controller.menuOpen && controller.menuPlacement === 'up'}
    style="opacity: {effectiveOpacity}"
    ondblclick={handleDoubleClick}
    onwheel={handleWheel}
    title={barTitle}
>
    <div
        class="minitracker-bar"
        bind:this={barElement}
        style:height={controller.menuOpen ? `${menuBarHeight}px` : '100%'}
    >
        <span
            class="minitracker-time"
            style="color: {textColor}"
            title="Drag to move"
            onpointerdown={controller.pointerDown}
            onpointermove={controller.pointerMove}
            onpointerup={controller.pointerUp}
            onpointercancel={controller.pointerUp}
        >{timeParts.m}:{timeParts.s}</span>

        <button
            type="button"
            class="minitracker-task"
            class:has-task={activeTask !== undefined}
            aria-label={activeTask ? `Current task: ${activeTask.title}` : 'Select a task'}
            aria-expanded={controller.menuOpen}
            title={activeTask ? activeTask.title : 'Select a task'}
            onclick={() => void handleToggleMenu()}
        >
            {activeTask?.title ?? 'select task'}
        </button>
    </div>

    {#if controller.menuOpen}
        <TaskMenu
            {tasks}
            {activeTask}
            onSelect={(taskId) => void selectTask(taskId)}
            onStop={() => void stopWorking()}
            onToggleDone={(task) => void toggleDone(task)}
        />
    {/if}
</div>
