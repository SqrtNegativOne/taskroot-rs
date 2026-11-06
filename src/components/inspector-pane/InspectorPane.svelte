<script lang="ts">
    import { onMount } from 'svelte';
    import './inspector.css';
    import Icon from '../Icon.svelte';
    import DescriptionInput from '../inputs/DescriptionInput.svelte';
    import InspectorEventFields from './InspectorEventFields.svelte';
    import InspectorField from './InspectorField.svelte';
    import InspectorTaskFields from './InspectorTaskFields.svelte';
    import InspectorTitle from './InspectorTitle.svelte';
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
        function handlePointerDown(e: PointerEvent) {
            if (inspector.recurringModalOpen) return;
            if (inspectorState && paneRef && e.target instanceof Node && !paneRef.contains(e.target)) {
                onClose();
            }
        }

        function handleKeyDown(e: KeyboardEvent) {
            if (e.key !== 'Escape' || inspector.recurringModalOpen) return;
            if (!currentItem) return;
            e.preventDefault();
            onClose();
        }

        document.addEventListener('pointerdown', handlePointerDown);
        document.addEventListener('keydown', handleKeyDown);
        return () => {
            document.removeEventListener('pointerdown', handlePointerDown);
            document.removeEventListener('keydown', handleKeyDown);
        };
    });
</script>

<div
    bind:this={paneRef}
    class="inspector-pane"
    class:is-open={currentItem !== undefined}
    aria-hidden={currentItem === undefined}
>
    {#if currentItem}
        <header class="inspector-hd">
            <span class="inspector-kind" aria-hidden="true">
                <Icon name={isCurrentTask ? 'task_alt' : 'event'} size={16} />
                {isCurrentTask ? 'Task' : 'Event'}
            </span>
            <InspectorTitle
                value={currentItem.title}
                onchange={inspector.handleTitleChange}
                disabled={isReadOnly}
            />
        </header>

        <div class="inspector-body">
            <InspectorField icon="notes" label="Description" multiline>
                <DescriptionInput
                    value={currentTask ? currentTask.notes : currentEvent?.description}
                    onchange={inspector.handleDescriptionChange}
                    disabled={isReadOnly}
                />
            </InspectorField>

            {#if currentTask}
                <InspectorTaskFields
                    task={currentTask}
                    tasks={inspector.tasks}
                    updateTask={inspector.updateTask}
                    readOnly={isReadOnly}
                />
            {:else if currentEvent}
                <InspectorEventFields
                    event={currentEvent}
                    tasks={inspector.tasks}
                    updateEvent={inspector.updateEvent}
                    readOnly={isReadOnly}
                />
            {/if}

            <footer class="inspector-actions">
                <button
                    type="button"
                    class="inspector-delete"
                    onclick={inspector.handleDelete}
                    disabled={isReadOnly}
                >
                    <Icon name="delete" size={16} />
                    Delete {isCurrentTask ? 'task' : 'event'}
                </button>
                {#if isReadOnly}
                    <p class="inspector-readonly-note">
                        This item is in a read-only calendar.
                    </p>
                {/if}
            </footer>
        </div>
    {/if}
</div>

<RecurringActionModal
    isOpen={inspector.recurringModalOpen}
    actionType="delete"
    onConfirm={inspector.handleRecurringConfirm}
    onCancel={inspector.handleRecurringCancel}
/>
