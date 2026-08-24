<script lang="ts">
    import { onMount } from 'svelte';
    import './inspector.css';
    import type { AppTask, AppEvent } from '../../lib/domain';
    import InspectorTaskFields from './InspectorTaskFields.svelte';
    import InspectorEventFields from './InspectorEventFields.svelte';
    import DescriptionInput from '../inputs/DescriptionInput.svelte';
    import TitleInput from '../inputs/TitleInput.svelte';

    let {
        inspectorState,
        onClose,
        tasks,
        events,
        updateTask,
        updateEvent,
        deleteTask,
        deleteEvent
    }: {
        inspectorState?: { type: 'task' | 'event'; id: string };
        onClose: () => void;
        tasks: AppTask[];
        events: AppEvent[];
        updateTask: (id: string, t: (t: AppTask) => AppTask) => void;
        updateEvent: (id: string, e: (e: AppEvent) => AppEvent) => void;
        deleteTask: (id: string) => void;
        deleteEvent: (id: string) => void;
    } = $props();

    let paneRef = $state<HTMLElement | null>(null);

    let currentTask = $derived(
        inspectorState?.type === 'task' ? tasks.find(t => t.id === inspectorState.id) : undefined
    );
    let currentEvent = $derived(
        inspectorState?.type === 'event' ? events.find(e => e.id === inspectorState.id) : undefined
    );
    let currentItem = $derived(currentTask ?? currentEvent);
    let isCurrentTask = $derived(currentTask !== undefined);

    onMount(() => {
        function handleClickOutside(e: PointerEvent) {
            if (inspectorState && paneRef && e.target instanceof Node && !paneRef.contains(e.target)) {
                onClose();
            }
        }
        document.addEventListener('pointerdown', handleClickOutside);
        return () => { document.removeEventListener('pointerdown', handleClickOutside); };
    });

    function handleTitleChange(value: string): void {
        if (currentTask) updateTask(currentTask.id, (t) => ({ ...t, title: value }));
        else if (currentEvent) updateEvent(currentEvent.id, (ev) => ({ ...ev, title: value }));
    }

    function handleDescriptionChange(value: string): void {
        if (currentTask) updateTask(currentTask.id, (t) => ({ ...t, notes: value }));
        else if (currentEvent) updateEvent(currentEvent.id, (ev) => ({ ...ev, description: value }));
    }

    function handleDelete(): void {
        if (currentTask) deleteTask(currentTask.id);
        else if (currentEvent) deleteEvent(currentEvent.id);
        onClose();
    }
</script>

<div
    bind:this={paneRef}
    class="inspector-pane"
    class:is-open={currentItem !== undefined}
>
    {#if currentItem}
        <header class="inspector-hd">
            <TitleInput
                value={currentItem.title}
                onchange={handleTitleChange}
                class="inspector-title-input"
            />
            <button class="icon-btn" onclick={onClose}>✕</button>
        </header>

        <div class="inspector-body">
            <div class="inspector-field">
                <DescriptionInput
                    value={currentTask ? currentTask.notes : currentEvent?.description}
                    onchange={handleDescriptionChange}
                    class="inspector-desc-input"
                />
            </div>

            {#if currentTask}
                <InspectorTaskFields task={currentTask} {updateTask} />
            {:else if currentEvent}
                <InspectorEventFields event={currentEvent} {tasks} {updateEvent} />
            {/if}

            <div class="inspector-actions" style="margin-top: 24px;">
                <button class="btn-danger" onclick={handleDelete}>
                    Delete {isCurrentTask ? 'Task' : 'Event'}
                </button>
            </div>
        </div>
    {/if}
</div>
