<script lang="ts">
    import { onMount } from 'svelte';
    import './inspector.css';
    import type { AppTask, AppEvent, AppCalendar } from '../../lib/domain';
    import { isEventReadOnly } from '../../lib/domain';
    import InspectorTaskFields from './InspectorTaskFields.svelte';
    import InspectorEventFields from './InspectorEventFields.svelte';
    import DescriptionInput from '../inputs/DescriptionInput.svelte';
    import TitleInput from '../inputs/TitleInput.svelte';
    import RecurringActionModal, { type RecurringMode } from '../RecurringActionModal.svelte';

    import { useAutoQuery } from '../../lib/safeInvoke.svelte';
    import { store } from '../../lib/store.svelte';

    let {
        inspectorState,
        onClose,
    }: {
        inspectorState?: { type: 'task' | 'event'; id: string; occurrenceStart?: string };
        onClose: () => void;
    } = $props();

    const tasksQuery = useAutoQuery<AppTask[]>('query_tasks', () => ({ filters: [], sort: [], query: "" }));
    const eventsQuery = useAutoQuery<AppEvent[]>('query_events', () => ({ filters: [], query: "" }));
    const calendarsQuery = useAutoQuery<AppCalendar[]>('get_active_calendars', () => ({}));

    let tasks = $derived(tasksQuery.data ?? []);
    let events = $derived(eventsQuery.data ?? []);

    let recurringModalOpen = $state(false);
    let eventPendingAction = $state<AppEvent | undefined>(undefined);
    /** The clicked occurrence's canonical value, absent when the series itself was opened. */
    let eventPendingStart = $state<string | undefined>(undefined);

    function handleRecurringConfirm(mode: RecurringMode) {
        recurringModalOpen = false;
        if (!eventPendingAction) {
            onClose();
            return;
        }

        const id = eventPendingAction.id;
        const occurrenceStart = eventPendingStart;
        eventPendingAction = undefined;
        eventPendingStart = undefined;

        void store.mutate('Failed to delete event', () =>
            store.deleteEvent(id, mode, occurrenceStart),
        );
        onClose();
    }

    function handleRecurringCancel() {
        recurringModalOpen = false;
        eventPendingAction = undefined;
        eventPendingStart = undefined;
    }

    function updateTask(id: string, t: (t: AppTask) => AppTask) {
        const current = tasks.find(x => x.id === id);
        if (current) {
            void store.mutate('Failed to update task', () => store.updateTask(t(current)));
        }
    }

    function updateEvent(id: string, e: (e: AppEvent) => AppEvent) {
        const current = events.find(x => x.id === id);
        if (current) {
            void store.mutate('Failed to update event', () => store.updateEvent(e(current)));
        }
    }

    function deleteTask(id: string) {
        void store.mutate('Failed to delete task', () => store.deleteTask(id));
    }

    function deleteEvent(id: string) {
        void store.mutate('Failed to delete event', () => store.deleteEvent(id, 'all'));
    }

    let paneRef = $state<HTMLElement | null>(null);

    let currentTask = $derived(
        inspectorState?.type === 'task' ? tasks.find(t => t.id === inspectorState.id) : undefined
    );
    let currentEvent = $derived(
        inspectorState?.type === 'event' ? events.find(e => e.id === inspectorState.id) : undefined
    );
    let currentItem = $derived(currentTask ?? currentEvent);
    let isCurrentTask = $derived(currentTask !== undefined);
    let isReadOnly = $derived(
        currentEvent !== undefined && isEventReadOnly(currentEvent, calendarsQuery.data ?? [])
    );

    onMount(() => {
        function handleClickOutside(e: PointerEvent) {
            if (recurringModalOpen) return;
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
        if (isReadOnly) return;
        if (currentTask) {
            deleteTask(currentTask.id);
            onClose();
        } else if (currentEvent) {
            if (currentEvent.rrule || currentEvent.recurringEventId) {
                eventPendingAction = currentEvent;
                eventPendingStart = inspectorState?.occurrenceStart;
                recurringModalOpen = true;
            } else {
                deleteEvent(currentEvent.id);
                onClose();
            }
        }
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
                disabled={isReadOnly}
                class="inspector-title-input"
            />
            <button class="icon-btn" onclick={onClose}>✕</button>
        </header>

        <div class="inspector-body">
            <div class="inspector-field">
                <DescriptionInput
                    value={currentTask ? currentTask.notes : currentEvent?.description}
                    onchange={handleDescriptionChange}
                    disabled={isReadOnly}
                    class="inspector-desc-input"
                />
            </div>

            {#if currentTask}
                <InspectorTaskFields task={currentTask} {tasks} {updateTask} />
            {:else if currentEvent}
                <InspectorEventFields event={currentEvent} {tasks} {updateEvent} readOnly={isReadOnly} />
            {/if}

            <div class="inspector-actions" style="margin-top: 24px;">
                <button class="btn-danger" onclick={handleDelete} disabled={isReadOnly} style={isReadOnly ? 'opacity: 0.5; cursor: not-allowed;' : ''}>
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
    isOpen={recurringModalOpen}
    actionType="delete"
    onConfirm={handleRecurringConfirm}
    onCancel={handleRecurringCancel}
/>
