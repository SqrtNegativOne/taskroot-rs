<script lang="ts">
    import { onMount } from 'svelte';
    import type { AppTask } from '../../lib/domain';

    let {
        task,
        x,
        y,
        onClose,
        onDelete,
        onDo,
    }: {
        task: AppTask;
        x: number;
        y: number;
        onClose: () => void;
        onDelete: (id: string) => void;
        onDo: (task: AppTask) => void;
    } = $props();

    let menuEl: HTMLDivElement | undefined = $state();

    function handlePointerDown(e: PointerEvent) {
        if (e.target instanceof Node && menuEl?.contains(e.target)) return;
        onClose();
    }

    function handleKeyDown(e: KeyboardEvent) {
        if (e.key === 'Escape') onClose();
    }

    function handleDismiss() {
        onClose();
    }

    onMount(() => {
        window.addEventListener('pointerdown', handlePointerDown, true);
        window.addEventListener('keydown', handleKeyDown);
        window.addEventListener('scroll', handleDismiss, true);
        window.addEventListener('resize', handleDismiss);
        return () => {
            window.removeEventListener('pointerdown', handlePointerDown, true);
            window.removeEventListener('keydown', handleKeyDown);
            window.removeEventListener('scroll', handleDismiss, true);
            window.removeEventListener('resize', handleDismiss);
        };
    });

    function doTask() {
        onDo(task);
        onClose();
    }

    function deleteTask() {
        onDelete(task.id);
        onClose();
    }
</script>

<div
    bind:this={menuEl}
    class="task-context-menu"
    role="menu"
    tabindex="-1"
    aria-label={task.title}
    style="left: min({x}px, calc(100vw - 190px)); top: min({y}px, calc(100vh - 130px));"
    oncontextmenu={(e) => e.preventDefault()}
>
    <div class="task-context-title" title={task.title}>{task.title}</div>
    <button type="button" role="menuitem" class="task-context-item" onclick={doTask}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M8 5v14l11-7z" /></svg>
        Do
    </button>
    <button type="button" role="menuitem" class="task-context-item is-danger" onclick={deleteTask}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
        Delete
    </button>
</div>

<style>
    .task-context-menu {
        position: fixed;
        z-index: 1000;
        min-width: 170px;
        display: flex;
        flex-direction: column;
        padding: 4px;
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-radius: 6px;
        box-shadow: 0 6px 18px rgba(0, 0, 0, 0.3);
        font-family: var(--sans);
        font-size: calc(13px * var(--font-scale, 1));
    }

    .task-context-title {
        padding: 5px 8px 6px;
        margin-bottom: 4px;
        border-bottom: 1px dashed var(--border-soft);
        color: var(--fg-dim);
        max-width: 240px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .task-context-item {
        display: flex;
        align-items: center;
        gap: 8px;
        width: 100%;
        padding: 6px 8px;
        background: none;
        border: 0;
        border-radius: 4px;
        color: var(--fg);
        font: inherit;
        text-align: left;
        cursor: pointer;
    }

    .task-context-item:hover,
    .task-context-item:focus-visible {
        background: var(--bg-surface-hover);
        outline: none;
    }

    .task-context-item.is-danger {
        color: var(--p0);
    }
</style>
