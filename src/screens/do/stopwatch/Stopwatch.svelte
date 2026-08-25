<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { stopwatchState } from './engine.svelte';
    import StopwatchDisplay from './StopwatchDisplay.svelte';
    import ActiveTaskDisplay from './ActiveTaskDisplay.svelte';
    import TaskSelector from '../TaskSelector.svelte';
    import { store, describeError } from '../../../lib/store.svelte';
    import type { AppTaskStatus } from '../../../lib/domain';
    import './stopwatch.css';

    let { onBreakStatusChange }: { onBreakStatusChange?: (status: boolean) => void; } = $props();

    let selectorOpen = $state(false);
    let allowNoTask = $derived(store.settings?.allow_stopwatch_without_task ?? false);

    let activeTask = $derived(store.tasks.find((task) => task.status === 'doing'));

    // If no active task and we require one, open the selector
    $effect(() => {
        if (!activeTask && !allowNoTask) {
            selectorOpen = true;
        }
    });

    // Notify break status changes
    $effect(() => {
        onBreakStatusChange?.(stopwatchState.isBreak);
    });

    async function updateTaskStatus(taskId: string, status: AppTaskStatus): Promise<void> {
        const result = await store.updateTask(taskId, (task) => ({ ...task, status }));
        if (result.isErr()) {
            store.error = `Failed to update task status: ${describeError(result.error)}`;
        }
    }

    async function setActiveTask(taskId: string): Promise<void> {
        for (const task of store.tasks) {
            if (task.id === taskId) {
                await updateTaskStatus(task.id, 'doing');
            } else if (task.status === 'doing') {
                await updateTaskStatus(task.id, 'todo');
            }
        }
        selectorOpen = false;

        if (!stopwatchState.running) {
            await stopwatchState.toggle();
        }
    }

    async function toggleStopwatch(): Promise<void> {
        if (stopwatchState.isPristine && !activeTask && !allowNoTask) {
            selectorOpen = true;
            return;
        }
        await stopwatchState.toggle();
    }

    async function openMinitracker(): Promise<void> {
        const { safeInvoke } = await import('../../../lib/safeInvoke.svelte');
        const result = await safeInvoke('show_minitracker');
        if (result.isErr()) {
            store.error = `Failed to open mini tracker: ${describeError(result.error)}`;
        }
    }

    // Keyboard shortcuts
    function handleKeydown(e: KeyboardEvent) {
        if (e.target instanceof HTMLElement && e.target.matches('input:not(.task-search-input), textarea, [contenteditable]')) {
            return;
        }

        const isModifier = e.metaKey || e.ctrlKey;

        switch (e.code) {
            case "Space":
                if (!selectorOpen) {
                    e.preventDefault();
                    void toggleStopwatch();
                }
                break;
            case "KeyR":
                if (isModifier) {
                    e.preventDefault();
                    void stopwatchState.reset();
                }
                break;
            case "Enter":
                if (isModifier) {
                    e.preventDefault();
                    selectorOpen = !selectorOpen;
                }
                break;
            case "Escape":
                if (selectorOpen && (activeTask || allowNoTask)) {
                    e.preventDefault();
                    selectorOpen = false;
                }
                break;
        }
    }

    onMount(() => {
        window.addEventListener('keydown', handleKeydown);
    });

    onDestroy(() => {
        window.removeEventListener('keydown', handleKeydown);
    });
</script>

<section class="stopwatch-hero">
    <div class="stopwatch-stage" style="position: relative;">
        <StopwatchDisplay
            engine={stopwatchState}
            onToggle={() => { void toggleStopwatch(); }}
        />
        <div style="position: absolute; right: 0; top: 0; display: flex; flex-direction: column; align-items: flex-end; gap: 4px;">
            <button onclick={() => { void openMinitracker(); }} class="sw-btn" style="padding: 4px 8px; font-size: 12px; margin-top: 8px; margin-right: 8px;" title="Open Mini Tracker">
                Mini Tracker
            </button>
            {#if store.settings?.clock_style === 'flowtime'}
                <button onclick={() => { void stopwatchState.toggleBreak(); }} class="sw-btn" style="padding: 4px 8px; font-size: 12px; margin-right: 8px;" title="Toggle Break">
                    {stopwatchState.isBreak ? 'End Break' : 'Take Break'}
                </button>
            {/if}
        </div>

        {#if (stopwatchState.running || stopwatchState.isBreak) && (activeTask ?? allowNoTask)}
            <ActiveTaskDisplay
                activeTask={activeTask}
                onOpenSelector={() => selectorOpen = true}
            />
        {/if}

        <TaskSelector
            selectorOpen={selectorOpen}
            onCloseSelector={() => selectorOpen = false}
            tasks={store.tasks}
            activeTask={activeTask}
            onStartWithTask={(taskId: string) => { void setActiveTask(taskId); }}
        />
    </div>
</section>
