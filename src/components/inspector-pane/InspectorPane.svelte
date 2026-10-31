<script lang="ts">
    import { onMount } from 'svelte';
    import './inspector.css';
    import InspectorTaskFields from './InspectorTaskFields.svelte';
    import InspectorEventFields from './InspectorEventFields.svelte';
    import DescriptionInput from '../inputs/DescriptionInput.svelte';
    import TitleInput from '../inputs/TitleInput.svelte';
    import RecurringActionModal from '../RecurringActionModal.svelte';
    import { createInspectorState, type InspectorTarget } from './state.svelte';

    let {
        inspectorState,
        onClose,
    }: {
        inspectorState?: InspectorTarget;
        onClose: () => void;
    } = $props();

    const inspector = createInspectorState(() => inspectorState, () => onClose());

    const currentTask = $derived(inspector.currentTask);
    const currentEvent = $derived(inspector.currentEvent);
    const currentItem = $derived(inspector.currentItem);
    const isCurrentTask = $derived(inspector.isCurrentTask);
    const isReadOnly = $derived(inspector.isReadOnly);

    let paneRef = $state<HTMLElement | null>(null);

    onMount(() => {
        function handleClickOutside(e: PointerEvent) {
            if (inspector.recurringModalOpen) return;
            if (inspectorState && paneRef && e.target instanceof Node && !paneRef.contains(e.target)) {
                onClose();
            }
        }
        document.addEventListener('pointerdown', handleClickOutside);
        return () => { document.removeEventListener('pointerdown', handleClickOutside); };
    });
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
                onchange={inspector.handleTitleChange}
                disabled={isReadOnly}
                class="inspector-title-input"
            />
            <button class="icon-btn" onclick={onClose}>✕</button>
        </header>

        <div class="inspector-body">
            <div class="inspector-field">
                <DescriptionInput
                    value={currentTask ? currentTask.notes : currentEvent?.description}
                    onchange={inspector.handleDescriptionChange}
                    disabled={isReadOnly}
                    class="inspector-desc-input"
                />
            </div>

            {#if currentTask}
                <InspectorTaskFields task={currentTask} tasks={inspector.tasks} updateTask={inspector.updateTask} />
            {:else if currentEvent}
                <InspectorEventFields event={currentEvent} tasks={inspector.tasks} updateEvent={inspector.updateEvent} readOnly={isReadOnly} />
            {/if}

            <div class="inspector-actions" style="margin-top: 24px;">
                <button class="btn-danger" onclick={inspector.handleDelete} disabled={isReadOnly} style={isReadOnly ? 'opacity: 0.5; cursor: not-allowed;' : ''}>
                    Delete {isCurrentTask ? 'Task' : 'Event'}
                </button>
                {#if isReadOnly}
                    <p style="margin: 8px 0 0; font-size: 12px; color: var(--fg-dim);">
                        This event is in a read-only calendar.
                    </p>
                {/if}
            </div>
        </div>
    {/if}
</div>

<RecurringActionModal
    isOpen={inspector.recurringModalOpen}
    actionType="delete"
    onConfirm={inspector.handleRecurringConfirm}
    onCancel={inspector.handleRecurringCancel}
/>
