<script lang="ts">
    import { onMount } from 'svelte';
    import { getCurrentWindow, LogicalPosition } from '@tauri-apps/api/window';
    import { store } from '../../lib/store.svelte';
    import { commands, safeInvoke, useAutoQuery } from '../../lib/safeInvoke.svelte';
    import { persistState, persistedKeys } from '../../lib/persisted.svelte';
    import { allTasks as allTasksQueryArgs } from '../../lib/queries';
    import { reloadTaskListState, taskListQueryArgs } from '../../lib/taskList.svelte';
    import { stopwatchState, splitTime } from '../../screens/do/stopwatch/engine.svelte';
    import TaskMenu from './TaskMenu.svelte';
    import {
        createMinitrackerController,
        isInteractiveTarget,
        isStoredPosition,
        isTaskMenuTarget,
        type StoredPosition,
    } from './controller.svelte';
    import { createMinitrackerActions } from './actions';
    import {
        isTypingTarget,
        resolveMinitrackerKeyCommand,
        ShiftChordTracker,
    } from './shortcuts';
    import { DEFAULT_BAR_HEIGHT } from './menuLayout';
    import './minitracker.css';

    const PHASE_COLORS = {
        paused: 'var(--fg-muted)',
        running: 'var(--fg)',
        breaking: 'var(--success)',
        stopped: 'var(--fg-muted)',
    } as const;

    const breakDivisor = $derived(store.settings?.flowtime_break_divisor ?? 5);
    const pauseMinutes = $derived(store.settings?.pause_minutes ?? 10);
    let isDimmed = $state(false);
    let baseAlpha = $derived(
        store.settings?.tracker_opacity !== undefined ? store.settings.tracker_opacity / 100 : 0.85,
    );
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

    const actions = createMinitrackerActions({
        closeMenu: () => controller.closeMenu(),
        getPauseMinutes: () => pauseMinutes,
        getBreakDivisor: () => breakDivisor,
        toggleDimmed: () => {
            isDimmed = !isDimmed;
        },
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
        void safeInvoke(commands.updateSetting('tracker_opacity', percentage));
    }

    function handleWheel(event: WheelEvent): void {
        if (isTaskMenuTarget(event.target)) return;
        event.preventDefault();
        adjustOpacity(event.deltaY < 0 ? 0.05 : -0.05);
    }

    let allTasksQuery = useAutoQuery(commands.queryTasks, allTasksQueryArgs);
    let allTasks = $derived(allTasksQuery.data ?? []);

    let listQuery = useAutoQuery(commands.queryTasks, () => taskListQueryArgs(), { debounceMs: 150 });
    let tasks = $derived(listQuery.data ?? []);

    let activeTask = $derived(allTasks.find((task) => task.id === stopwatchState.taskId));
    let timeParts = $derived(splitTime(stopwatchState.currentMs));
    let timeText = $derived(stopwatchState.isBreak ? timeParts.sec : timeParts.m);
    let textColor = $derived(PHASE_COLORS[stopwatchState.phase]);
    let customLabel = $derived(
        stopwatchState.labelSource === 'custom' ? stopwatchState.label : undefined,
    );
    let hasCustomLabel = $derived(customLabel !== undefined && customLabel !== '');
    let hasTaskLabel = $derived(stopwatchState.labelSource === 'task' && !hasCustomLabel);
    let barLabel = $derived(
        hasCustomLabel
            ? (customLabel ?? '')
            : (activeTask?.title ?? stopwatchState.label ?? 'select task'),
    );

    const barTitle = $derived(
        hasTaskLabel
            ? `Tracking ${barLabel}. Drag the clock to move, click the label to switch. Double-click to quit.`
            : hasCustomLabel
                ? `Tracking ${customLabel ?? ''}. Drag the clock to move, click the label to change it. Double-click to quit.`
                : 'No active label. Drag the clock to move, click the label to choose a task or type your own. Double-click to quit.',
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
        void controller.toggleMenu(
            tasks.length + (activeTask ? 1 : 0),
            { width: rect.width, height: rect.height },
        );
    }

    onMount(() => {
        void getCurrentWindow().setAlwaysOnTop(true);
        const shiftChord = new ShiftChordTracker();

        const handleKeyDown = (e: KeyboardEvent) => {
            const cmd = resolveMinitrackerKeyCommand(
                e,
                {
                    isTyping: isTypingTarget(e.target),
                    menuOpen: controller.menuOpen,
                    isPaused: stopwatchState.isPaused,
                },
                shiftChord,
            );
            if (cmd === null) return;
            e.preventDefault();
            actions.executeKeyCommand(cmd);
        };
        const handleKeyUp = (e: KeyboardEvent) => shiftChord.onKeyUp(e.code);
        const handleBlur = () => {
            shiftChord.reset();
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
        await safeInvoke(commands.windowRestoreMain());
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
        >{timeText}</span>

        <button
            type="button"
            class="minitracker-task"
            class:has-task={hasTaskLabel}
            class:has-custom={hasCustomLabel}
            aria-label={hasCustomLabel ? `Tracking: ${customLabel ?? ''}` : hasTaskLabel ? `Current task: ${activeTask?.title ?? ''}` : 'Select a task or type a label'}
            aria-expanded={controller.menuOpen}
            title={barLabel}
            onclick={() => void handleToggleMenu()}
        >
            {barLabel}
        </button>
    </div>

    {#if controller.menuOpen}
        <TaskMenu
            {tasks}
            {activeTask}
            customLabel={hasCustomLabel ? customLabel : undefined}
            onSelect={(taskId) => void actions.selectTask(tasks, taskId)}
            onStop={() => void actions.stopWorking(activeTask)}
            onSetCustom={(label) => void actions.setCustomLabel(label)}
            onToggleDone={(task) => void actions.toggleDone(task)}
            onAddTask={(title) => void actions.addAndTrackTask(title)}
        />
    {/if}
</div>
