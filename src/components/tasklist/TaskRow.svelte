<script lang="ts">
    import TaskCircle from '../TaskCircle.svelte';
    import Icon from '../Icon.svelte';
    import { isRecurring, type AppTask, type TaskPatch } from '../../lib/domain';
    import { ymd } from '../../lib/time';

    let {
        task,
        onDragStart,
        dragging,
        updateTask,
        isPastDue,
        selected,
        onTaskClick,
        onContextMenu,
        isActive,
    }: {
        task: AppTask;
        onDragStart?: (e: PointerEvent | MouseEvent, task: AppTask) => void;
        dragging?: boolean;
        updateTask: (id: string, patches: TaskPatch[]) => void;
        isPastDue?: boolean;
        selected?: boolean;
        onTaskClick?: (task: AppTask) => void;
        onContextMenu?: (e: MouseEvent, task: AppTask) => void;
        isActive?: boolean;
    } = $props();

    let isChecking = $state(false);

    function handlePointerDown(e: PointerEvent | MouseEvent) {
        if (e.button !== 0) return;
        if (e.target instanceof Element && (
            e.target.closest('.task-row-subtask-toggle') ||
            e.target.closest('.task-circle')
        )) return;
        if (onDragStart) onDragStart(e, task);
    }

    function handleContextMenu(e: MouseEvent) {
        e.preventDefault();
        onContextMenu?.(e, task);
    }
    
    function handleCircleClick(e: MouseEvent) {
        e.stopPropagation();
        const newStatus = task.status === "done" ? "todo" : "done";
        
        if (newStatus === "done") {
            isChecking = true;
            // sound effect play here
        }

        updateTask(task.id, [{ field: 'status', value: newStatus }]);
        if (newStatus === "todo") {
            isChecking = false;
        }
    }
    
    let dueStr = $derived(task.due ? `due ${task.due}` : "");
    let overdue = $derived(
        !!task.due
        && task.due.slice(0, 10) < ymd(new Date())
        && task.status !== "done",
    );

    let startState = $derived.by(() => {
        if (!task.start) return null;
        const today = ymd(new Date());
        const day = task.start.slice(0, 10);
        if (day < today) return "past";
        if (day === today) return "today";
        return "future";
    });

    let startTitle = $derived(
        startState === "past" ? "Start date passed"
        : startState === "today" ? "Starts today"
        : "Scheduled start",
    );

    let hasNotes = $derived(!!task.notes && task.notes.trim() !== "");
    let hasTags = $derived(!!task.tags && task.tags.length > 0);
    let isTaskRecurring = $derived(isRecurring({ rrule: task.rrule }));
    let est = $derived(task.est ?? 0);
    let hasEst = $derived(est > 0);
</script>

<div
    class="task-row"
    class:is-dragging={dragging}
    class:is-done={task.status === "done"}
    class:is-selected={selected}
    onpointerdown={handlePointerDown}
    oncontextmenu={handleContextMenu}
    onclick={(e) => {
        if (e.target instanceof Element && (
            e.target.closest('.task-row-subtask-toggle') ||
            e.target.closest('.task-circle')
        )) return;
        if (onTaskClick) onTaskClick(task);
    }}
>
    <TaskCircle
        priority={task.priority ?? undefined}
        isDoneOrChecking={task.status === "done" || isChecking}
        isActive={isActive}
        onclick={handleCircleClick}
    />
    <div class="task-row-content">
        <div class="task-row-line1">
            {#if isPastDue && task.status !== "done"}
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="color: var(--p0); flex-shrink: 0;">
                    <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
                </svg>
            {/if}
            {#if isTaskRecurring}
                <span class="task-row-recurring-icon" title="Recurring task">↻</span>
            {/if}
            <span class="task-row-title">{task.title}{#if hasNotes}<span class="task-row-notes-icon" title="Has a description"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M4 7h16M4 12h16M4 17h9"/></svg></span>{/if}{#if startState}<span
                    class="task-row-start-icon"
                    class:is-past={startState === "past"}
                    class:is-today={startState === "today"}
                    title={startTitle}
                ><Icon name="pending_actions" size={13} /></span>{/if}</span>
            {#if task.status === "next-up"}
                <span class="status-pill status-nextup">next up</span>
            {/if}
        </div>
        {#if hasEst || hasTags || dueStr}
            <div class="task-row-line2">
                {#if hasEst}
                    <span class="meta-est">{est}m</span>
                    {#if hasTags}<span class="meta-sep">·</span>{/if}
                {/if}
                {#if task.tags}
                    {#each task.tags as tag, i (tag.id)}
                        <span class="meta-tag">#{tag.name}</span>
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
