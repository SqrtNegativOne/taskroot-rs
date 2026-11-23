<script lang="ts">
    import { stopwatchState } from './engine.svelte';
    import StopwatchDisplay from './StopwatchDisplay.svelte';
    import ActiveTaskDisplay from './ActiveTaskDisplay.svelte';
    import { store } from '../../../lib/store.svelte';
    import './stopwatch.css';

    import { useTauriQuery } from '../../../lib/safeInvoke.svelte';
    import { allTasks as allTasksQueryArgs } from '../../../lib/queries';
    import type { AppTask } from '../../../lib/domain';

    let { onBreakStatusChange }: { onBreakStatusChange?: (status: boolean) => void; } = $props();

    const pauseMinutes = $derived(store.settings?.pause_minutes ?? 10);

    let allTasksQuery = useTauriQuery<AppTask[]>('query_tasks');
    let allTasks = $derived(allTasksQuery.data ?? []);

    $effect(() => {
        if (!store.loaded) return;
        void allTasksQuery.execute(allTasksQueryArgs());
    });

    let activeTask = $derived(
        stopwatchState.labelSource === 'task'
            ? allTasks.find((task) => task.id === stopwatchState.taskId)
            : undefined,
    );

    // Notify break status changes
    $effect(() => {
        onBreakStatusChange?.(stopwatchState.isBreak);
    });

    // P toggles the timed pause; arrow keys resize an active pause by a minute.
    function handlePauseKey(e: KeyboardEvent): boolean {
        if (e.code === "KeyP") {
            e.preventDefault();
            void stopwatchState.togglePause(pauseMinutes);
            return true;
        }
        if (!stopwatchState.isPaused) return false;
        if (e.code === "ArrowUp" || e.code === "ArrowRight") {
            e.preventDefault();
            void stopwatchState.adjustPause(1);
            return true;
        }
        if (e.code === "ArrowDown" || e.code === "ArrowLeft") {
            e.preventDefault();
            void stopwatchState.adjustPause(-1);
            return true;
        }
        return false;
    }

    // Keyboard shortcuts
    function handleKeydown(e: KeyboardEvent) {
        if (e.target instanceof HTMLElement && e.target.matches('input, textarea, [contenteditable]')) {
            return;
        }

        if (handlePauseKey(e)) return;

        if (e.code === "Space") {
            e.preventDefault();
            void stopwatchState.toggle();
            return;
        }

        if (e.code === "KeyR" && (e.metaKey || e.ctrlKey)) {
            e.preventDefault();
            void stopwatchState.reset();
        }
    }

</script>

<svelte:window onkeydown={handleKeydown} />

<section class="stopwatch-hero">
    <div class="stopwatch-stage" style="position: relative;">
        <StopwatchDisplay
            engine={stopwatchState}
            onToggle={() => { void stopwatchState.toggle(); }}
        />

        <ActiveTaskDisplay activeTask={activeTask} label={stopwatchState.label} />
    </div>
</section>
