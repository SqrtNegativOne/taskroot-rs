<script lang="ts">
    import { checkTaskAgainstFilters, type AppFilter } from './filters';
    import TaskCircle from '../TaskCircle.svelte';
    import type { AppTask, AppTaskStatus } from '../../lib/domain';

    const TRANSITION_DURATION_MS = 400;

    let {
        task,
        onDragStart,
        dragging,
        updateTask,
        deleteTask,
        filters,
        isPastDue,
    }: {
        task: AppTask;
        onDragStart?: (e: PointerEvent | MouseEvent, task: AppTask) => void;
        dragging?: boolean;
        updateTask: (id: string, transform: (task: AppTask) => AppTask) => void;
        deleteTask: (id: string) => void;
        filters: AppFilter[];
        isPastDue?: boolean;
    } = $props();

    let isExiting = $state(false);
    let isChecking = $state(false);

    function handlePointerDown(e: PointerEvent | MouseEvent) {
        if (e.button !== 0) return;
        if (e.target instanceof Element && (
            e.target.closest('.task-row-subtask-toggle') ||
            e.target.closest('.task-row-actions') ||
            e.target.closest('.task-circle')
        )) return;
        if (onDragStart) onDragStart(e, task);
    }
    
    function willBeFilteredOut(newStatus: AppTaskStatus) {
        return checkTaskAgainstFilters({ ...task, status: newStatus }, filters);
    }
    
    function handleCircleClick(e: MouseEvent) {
        e.stopPropagation();
        const newStatus = task.status === "done" ? "todo" : "done";
        const isRemoving = willBeFilteredOut(newStatus);
        
        if (newStatus === "done") {
            isChecking = true;
            // sound effect play here
        }

        if (isRemoving) {
            isExiting = true;
            setTimeout(() => {
                updateTask(task.id, t => ({ ...t, status: newStatus }));
                isChecking = false;
                isExiting = false;
            }, TRANSITION_DURATION_MS);
            return;
        }

        updateTask(task.id, t => ({ ...t, status: newStatus }));
        if (newStatus === "todo") {
            isChecking = false;
        }
    }
    
    function handleCircleContextMenu(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        if (task.status !== "doing") {
            updateTask(task.id, t => ({ ...t, status: 'doing' }));
        } else {
            updateTask(task.id, t => ({ ...t, status: 'todo' }));
        }
    }
    
    let dueStr = $derived(task.due ? `due ${task.due}` : "");
    let overdue = $derived(task.due && task.due < new Date().toISOString() && task.status !== "done");

    let hasTags = $derived(!!task.tags && task.tags.length > 0);
    let est = $derived(task.est ?? 0);
    let hasEst = $derived(est > 0);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class="task-row"
    class:is-dragging={dragging}
    class:is-done={task.status === "done"}
    class:is-exiting={isExiting}
    onpointerdown={handlePointerDown}
>
    <TaskCircle
        priority={task.priority ?? undefined}
        isDoneOrChecking={task.status === "done" || isChecking}
        isActive={task.status === "doing"}
        onclick={handleCircleClick}
        oncontextmenu={handleCircleContextMenu}
    />
    <div class="task-row-content">
        <div class="task-row-line1">
            <span class="task-row-title">
                {#if isPastDue && task.status !== "done"}
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-right: 4px; color: var(--p0); vertical-align: middle;">
                        <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
                    </svg>
                {/if}
                {task.title}
            </span>
            {#if task.status === "next-up"}
                <span class="status-pill status-nextup">next up</span>
            {/if}
            {#if task.status === "doing"}
                <span class="status-pill status-doing">DOING</span>
            {/if}
            <div class="task-row-actions">
                <button
                    onclick={(e) => {
                        e.stopPropagation();
                        if (e.shiftKey || confirm("Delete task?")) {
                            isExiting = true;
                            setTimeout(() => { deleteTask(task.id); }, TRANSITION_DURATION_MS);
                        }
                    }}
                    title="Delete"
                >
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
                </button>
            </div>
        </div>
        {#if hasEst || hasTags || dueStr}
            <div class="task-row-line2">
                {#if hasEst}
                    <span class="meta-est">{est}m</span>
                    {#if hasTags}<span class="meta-sep">·</span>{/if}
                {/if}
                {#if task.tags}
                    {#each task.tags as tag, i (tag)}
                        <span class="meta-tag">#{tag}</span>
                        {#if i < task.tags.length - 1}<span class="meta-tag-sep">,</span>{/if}
                    {/each}
                {/if}
                <span class="meta-spacer"></span>
                {#if dueStr}
                    <span class="meta-due" class:is-overdue={overdue}>{dueStr}</span>
                {/if}
            </div>
        {/if}
    </div>
</div>
