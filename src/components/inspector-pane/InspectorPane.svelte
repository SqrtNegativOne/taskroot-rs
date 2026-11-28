<script lang="ts">
    import { onMount } from 'svelte';
    import './inspector.css';
    import type { AppCalendar } from '../../lib/domain';
    import { notifyError } from '../../lib/toast';
    import DescriptionInput from '../inputs/DescriptionInput.svelte';
    import InspectorEventFields from './InspectorEventFields.svelte';
    import InspectorTaskFields from './InspectorTaskFields.svelte';
    import InspectorTitle from './InspectorTitle.svelte';
    import RecurringActionModal from '../RecurringActionModal.svelte';
    import { createInspectorState, type InspectorTarget } from './state.svelte';

    let {
        inspectorState,
        calendars,
        onClose,
        onConverted,
    }: {
        inspectorState?: InspectorTarget;
        calendars?: AppCalendar[];
        onClose: () => void;
        onConverted?: (target: InspectorTarget) => void;
    } = $props();

    const inspector = createInspectorState(
        () => inspectorState,
        () => onClose(),
        (task) => {
            if (onConverted) {
                onConverted({ type: 'task', id: task.id, draft: task });
                return;
            }
            onClose();
        },
        () => calendars,
    );

    const currentTask = $derived(inspector.currentTask);
    const currentEvent = $derived(inspector.currentEvent);
    const currentItem = $derived(inspector.currentItem);
    const isReadOnly = $derived(inspector.isReadOnly);

    let paneRef = $state<HTMLElement | null>(null);

    onMount(() => {
        function hasOpenPopover(): boolean {
            return document.querySelector('[data-inspector-popover]') !== null;
        }

        function handlePointerDown(e: PointerEvent) {
            if (inspector.recurringModalOpen || hasOpenPopover()) return;
            if (inspectorState && paneRef && e.target instanceof Node && !paneRef.contains(e.target)) {
                onClose();
            }
        }

        function handleKeyDown(e: KeyboardEvent) {
            if (e.key !== 'Escape' || e.defaultPrevented || inspector.recurringModalOpen || hasOpenPopover()) {
                return;
            }
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
            {#key currentItem.id}
                <InspectorTitle
                    value={currentItem.title}
                    onchange={inspector.handleTitleChange}
                    disabled={isReadOnly}
                    quickEdit={currentTask && !isReadOnly ? {
                        calendars: inspector.calendars,
                        onCommit: inspector.handleTaskQuickEdit,
                        onError: notifyError,
                    } : undefined}
                />
            {/key}
            <div class="inspector-description">
                <DescriptionInput
                    value={currentTask ? currentTask.notes : currentEvent?.description}
                    onchange={inspector.handleDescriptionChange}
                    disabled={isReadOnly}
                />
            </div>
        </header>

        <div class="inspector-body">
            {#if currentTask}
                <InspectorTaskFields
                    task={currentTask}
                    tasks={inspector.tasks}
                    calendars={inspector.calendars}
                    availableTags={inspector.availableTags}
                    updateTask={inspector.updateTask}
                    readOnly={isReadOnly}
                    onDelete={inspector.handleDelete}
                />
            {:else if currentEvent}
                <InspectorEventFields
                    event={currentEvent}
                    tasks={inspector.tasks}
                    calendars={inspector.calendars}
                    availableTags={inspector.availableTags}
                    updateEvent={inspector.updateEvent}
                    readOnly={isReadOnly}
                    onDelete={inspector.handleDelete}
                    onConvert={inspector.handleConvertToTask}
                />
            {/if}

            {#if isReadOnly}
                <p class="inspector-readonly-note">
                    This item is in a read-only calendar.
                </p>
            {/if}
        </div>
    {/if}
</div>

<RecurringActionModal
    isOpen={inspector.recurringModalOpen}
    actionType={inspector.recurringAction}
    onConfirm={inspector.handleRecurringConfirm}
    onCancel={inspector.handleRecurringCancel}
/>
