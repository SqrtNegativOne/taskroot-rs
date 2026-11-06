import type { AppCalendar, AppEvent, AppTask, EventScope } from '../../lib/domain';
import { isCalendarReadOnly, isEventReadOnly, isRecurring } from '../../lib/domain';
import { activeCalendars, allEvents } from '../../lib/queries';
import { useAutoQuery } from '../../lib/safeInvoke.svelte';
import { store } from '../../lib/store.svelte';

export type InspectorTarget =
    | { type: 'task'; id: string; draft?: AppTask }
    | { type: 'event'; id: string; occurrenceStart?: string; draft?: AppEvent };

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
        if (target?.type !== 'task') return undefined;
        return this.taskFor(target.id);
    }

    get currentEvent(): AppEvent | undefined {
        const target = this.getTarget();
        if (target?.type !== 'event') return undefined;
        return this.eventFor(target.id);
    }

    get currentItem(): AppTask | AppEvent | undefined {
        return this.currentTask ?? this.currentEvent;
    }

    get isCurrentTask(): boolean {
        return this.currentTask !== undefined;
    }

    get isReadOnly(): boolean {
        const calendars = this.calendarsQuery.data ?? [];
        const event = this.currentEvent;
        if (event !== undefined) return isEventReadOnly(event, calendars);

        const task = this.currentTask;
        if (task === undefined) return false;
        const calendar = calendars.find((candidate) => candidate.id === task.remoteCollectionId);
        return calendar !== undefined && isCalendarReadOnly(calendar);
    }

    /** The stored task, falling back to the draft carried by the target until the query catches up. */
    private taskFor(id: string): AppTask | undefined {
        const found = this.tasks.find((task) => task.id === id);
        if (found !== undefined) return found;
        const target = this.getTarget();
        return target?.type === 'task' && target.id === id ? target.draft : undefined;
    }

    /** The stored event, falling back to the draft carried by the target until the query catches up. */
    private eventFor(id: string): AppEvent | undefined {
        const found = this.events.find((event) => event.id === id);
        if (found !== undefined) return found;
        const target = this.getTarget();
        return target?.type === 'event' && target.id === id ? target.draft : undefined;
    }

    updateTask = (id: string, update: (task: AppTask) => AppTask): void => {
        const current = this.taskFor(id);
        if (!current) return;
        void store.mutate('Failed to update task', () => store.updateTask(update(current)));
    };

    updateEvent = (id: string, update: (event: AppEvent) => AppEvent): void => {
        const current = this.eventFor(id);
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
            const target = this.getTarget();
            this.eventPendingStart = target?.type === 'event' ? target.occurrenceStart : undefined;
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
