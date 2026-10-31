import type { AppCalendar, AppEvent, AppTask, EventScope } from '../../lib/domain';
import { isEventReadOnly, isRecurring } from '../../lib/domain';
import { activeCalendars, allEvents } from '../../lib/queries';
import { useAutoQuery } from '../../lib/safeInvoke.svelte';
import { store } from '../../lib/store.svelte';

export interface InspectorTarget {
    type: 'task' | 'event';
    id: string;
    occurrenceStart?: string;
}

/** Reactive inspector state: queries, the selected item, mutations and the recurring-delete scope prompt. */
export class InspectorPaneState {
    recurringModalOpen = $state(false);
    private eventPendingAction = $state<AppEvent | undefined>(undefined);
    private eventPendingStart = $state<string | undefined>(undefined);

    private readonly tasksQuery = useAutoQuery<AppTask[]>('query_tasks', () => ({ filters: [], sort: [], query: '' }));
    private readonly eventsQuery = useAutoQuery<AppEvent[]>('query_events', allEvents);
    private readonly calendarsQuery = useAutoQuery<AppCalendar[]>('get_active_calendars', activeCalendars);

    constructor(
        private readonly getTarget: () => InspectorTarget | undefined,
        private readonly onClose: () => void,
    ) {}

    get tasks(): AppTask[] {
        return this.tasksQuery.data ?? [];
    }

    get events(): AppEvent[] {
        return this.eventsQuery.data ?? [];
    }

    get currentTask(): AppTask | undefined {
        const target = this.getTarget();
        return target?.type === 'task' ? this.tasks.find((task) => task.id === target.id) : undefined;
    }

    get currentEvent(): AppEvent | undefined {
        const target = this.getTarget();
        return target?.type === 'event' ? this.events.find((event) => event.id === target.id) : undefined;
    }

    get currentItem(): AppTask | AppEvent | undefined {
        return this.currentTask ?? this.currentEvent;
    }

    get isCurrentTask(): boolean {
        return this.currentTask !== undefined;
    }

    get isReadOnly(): boolean {
        const event = this.currentEvent;
        return event !== undefined && isEventReadOnly(event, this.calendarsQuery.data ?? []);
    }

    updateTask = (id: string, update: (task: AppTask) => AppTask): void => {
        const current = this.tasks.find((task) => task.id === id);
        if (!current) return;
        void store.mutate('Failed to update task', () => store.updateTask(update(current)));
    };

    updateEvent = (id: string, update: (event: AppEvent) => AppEvent): void => {
        const current = this.events.find((event) => event.id === id);
        if (!current) return;
        void store.mutate('Failed to update event', () => store.updateEvent(update(current)));
    };

    handleTitleChange = (value: string): void => {
        const task = this.currentTask;
        if (task) {
            this.updateTask(task.id, (current) => ({ ...current, title: value }));
            return;
        }
        const event = this.currentEvent;
        if (event) this.updateEvent(event.id, (current) => ({ ...current, title: value }));
    };

    handleDescriptionChange = (value: string): void => {
        const task = this.currentTask;
        if (task) {
            this.updateTask(task.id, (current) => ({ ...current, notes: value }));
            return;
        }
        const event = this.currentEvent;
        if (event) this.updateEvent(event.id, (current) => ({ ...current, description: value }));
    };

    handleDelete = (): void => {
        if (this.isReadOnly) return;
        const task = this.currentTask;
        if (task) {
            void store.mutate('Failed to delete task', () => store.deleteTask(task.id));
            this.onClose();
            return;
        }
        const event = this.currentEvent;
        if (!event) return;
        if (isRecurring(event)) {
            this.eventPendingAction = event;
            this.eventPendingStart = this.getTarget()?.occurrenceStart;
            this.recurringModalOpen = true;
            return;
        }
        void store.mutate('Failed to delete event', () => store.deleteEvent(event.id, 'all'));
        this.onClose();
    };

    handleRecurringConfirm = (scope: EventScope): void => {
        this.recurringModalOpen = false;
        const pending = this.eventPendingAction;
        if (!pending) {
            this.onClose();
            return;
        }

        const occurrenceStart = this.eventPendingStart;
        this.eventPendingAction = undefined;
        this.eventPendingStart = undefined;

        void store.mutate('Failed to delete event', () =>
            store.deleteEvent(pending.id, scope, occurrenceStart),
        );
        this.onClose();
    };

    handleRecurringCancel = (): void => {
        this.recurringModalOpen = false;
        this.eventPendingAction = undefined;
        this.eventPendingStart = undefined;
    };
}

export function createInspectorState(
    getTarget: () => InspectorTarget | undefined,
    onClose: () => void,
): InspectorPaneState {
    return new InspectorPaneState(getTarget, onClose);
}
