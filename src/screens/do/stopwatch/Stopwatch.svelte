<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { stopwatchState } from './engine.svelte';
    import StopwatchDisplay from './StopwatchDisplay.svelte';
    import ActiveTaskDisplay from './ActiveTaskDisplay.svelte';
    import TaskSelector from '../TaskSelector.svelte';
    import { store } from '../../../lib/store.svelte';
    import './stopwatch.css';

    let { onBreakStatusChange } = $props<{
        onBreakStatusChange?: (status: boolean) => void;
    }>();

    let selectorOpen = $state(false);
    let allowNoTask = $state(true); // Using a default value for now until settings are ported

    let activeTask = $derived(store.tasks.find(t => t.status === 'doing'));

    // If no active task and we require one, open the selector
    $effect(() => {
        if (!activeTask && !allowNoTask) {
            selectorOpen = true;
        }
    });

    // Notify break status changes
    $effect(() => {
        if (onBreakStatusChange) {
            onBreakStatusChange(stopwatchState.isBreak);
        }
    });

    function toggleStopwatch() {
        if (stopwatchState.isPristine && !activeTask && !allowNoTask) {
            selectorOpen = true;
        } else {
            stopwatchState.toggle();
        }
    }

    async function openMinitracker() {
        const { safeInvoke } = await import('../../../lib/safeInvoke.svelte');
        await safeInvoke('show_minitracker');
    }

    function startWithTask(taskId: string) {
        // Find task and set to 'doing', set others to 'todo'
        // Since store doesn't have an updater for this easily, we loop
        for (const task of store.tasks) {
            if (task.id === taskId) {
                store.updateTask(task.id, t => ({ ...t, status: 'doing' }));
            } else if (task.status === 'doing') {
                store.updateTask(task.id, t => ({ ...t, status: 'todo' }));
            }
        }
        selectorOpen = false;
        
        // Start the stopwatch if it wasn't running
        if (!stopwatchState.running) {
            stopwatchState.toggle();
        }
    }

    // Keyboard shortcuts
    function handleKeydown(e: KeyboardEvent) {
        const target = e.target as HTMLElement;
        if (target && target.matches('input:not(.task-search-input), textarea, [contenteditable]')) {
            return;
        }

        const isModifier = e.metaKey || e.ctrlKey;

        switch (e.code) {
            case "Space":
                if (!selectorOpen) {
                    e.preventDefault();
                    toggleStopwatch();
                }
                break;
            case "KeyR":
                if (isModifier) {
                    e.preventDefault();
                    stopwatchState.reset();
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
            onToggle={toggleStopwatch} 
        />
        <div style="position: absolute; right: 0; top: 0;">
            <button onclick={openMinitracker} class="sw-btn" style="padding: 4px 8px; font-size: 12px; margin: 8px;" title="Open Mini Tracker">
                Mini Tracker
            </button>
        </div>

        {#if (stopwatchState.running || stopwatchState.isBreak) && (activeTask || allowNoTask)}
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
            onStartWithTask={startWithTask}
        />
    </div>
</section>
