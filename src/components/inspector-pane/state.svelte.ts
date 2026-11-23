import type { AppCalendar, AppEvent, AppTask, EventScope } from '../../lib/domain';
import {
    DEFAULT_TASK_PRIORITY,
    calendarsForComponent,
    isCalendarReadOnly,
    isEventReadOnly,
    isRecurring,
} from '../../lib/domain';
import { activeCalendars, allEvents } from '../../lib/queries';
import { useAutoQuery } from '../../lib/safeInvoke.svelte';
import { store } from '../../lib/store.svelte';
import { notifyError } from '../../lib/toast';
import { eventStartDate } from './eventDates';

export type InspectorTarget =
    | { type: 'task'; id: string; draft?: AppTask }
    | { type: 'event'; id: string; occurrenceStart?: string; draft?: AppEvent };

function writableTaskListId(
    calendars: readonly AppCalendar[],
    preferred?: string,
): string | undefined {
    const writable = calendarsForComponent(calendars, 'VTODO').filter(
        (calendar) => !isCalendarReadOnly(calendar),
    );
    if (preferred !== undefined && writable.some((calendar) => calendar.id === preferred)) {
        return preferred;
    }
    return writable[0]?.id;
}

/** Reactive inspector state: queries, the selected item, mutations and the recurring-delete scope prompt. */
export class InspectorPaneState {
    recurringModalOpen = $state(false);
    recurringAction = $state<'delete' | 'convert'>('delete');
    private eventPendingAction = $state<AppEvent | undefined>(undefined);
    private eventPendingStart = $state<string | undefined>(undefined);

    private readonly tasksQuery = useAutoQuery<AppTask[]>('query_tasks', () => ({ filters: [], sort: [], query: '' }));
    private readonly eventsQuery = useAutoQuery<AppEvent[]>('query_events', allEvents);
    private readonly calendarsQuery = useAutoQuery<AppCalendar[]>('get_active_calendars', activeCalendars);

    constructor(
        private readonly getTarget: () => InspectorTarget | undefined,
        private readonly onClose: () => void,
        private readonly onConverted: (task: AppTask) => void = onClose,
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
            this.recurringAction = 'delete';
            this.recurringModalOpen = true;
            return;
        }
        void store.mutate('Failed to delete event', () => store.deleteEvent(event.id, 'all'));
        this.onClose();
    };

    handleConvertToTask = (): void => {
        if (this.isReadOnly) return;
        const event = this.currentEvent;
        if (!event) return;
        const target = this.getTarget();
        const occurrenceStart = target?.type === 'event' ? target.occurrenceStart : undefined;

        if (isRecurring(event)) {
            this.eventPendingAction = event;
            this.eventPendingStart = occurrenceStart;
            this.recurringAction = 'convert';
            this.recurringModalOpen = true;
            return;
        }

        void this.convertEvent(event, 'all', undefined);
    };

    private async convertEvent(
        event: AppEvent,
        scope: EventScope,
        occurrenceStart: string | undefined,
    ): Promise<void> {
        const taskList = writableTaskListId(
            this.calendarsQuery.data ?? [],
            store.settings?.default_task_list,
        );
        if (taskList === undefined) {
            notifyError('Convert to task: no writable task list is available.');
            return;
        }

        const start = scope === 'all'
            ? eventStartDate(event)
            : occurrenceStart?.slice(0, 10) ?? eventStartDate(event);
        if (start === undefined) {
            notifyError('Convert to task: the event has no start date.');
            return;
        }

        const task: AppTask = {
            id: crypto.randomUUID(),
            title: event.title,
            status: 'todo',
            priority: event.priority ?? DEFAULT_TASK_PRIORITY,
            tags: event.tags?.map((tag) => ({ id: crypto.randomUUID(), name: tag.name })),
            notes: event.description,
            start,
            remoteCollectionId: taskList,
        };

        const created = await store.mutate('Failed to convert event to task', () => store.addTask(task));
        if (created.isErr()) return;

        const deleteStart = isRecurring(event) ? occurrenceStart : undefined;
        await store.mutate('Failed to remove converted event', () =>
            store.deleteEvent(event.id, scope, deleteStart),
        );
        this.onConverted(task);
    }

    handleRecurringConfirm = (scope: EventScope): void => {
        this.recurringModalOpen = false;
        const pending = this.eventPendingAction;
        const action = this.recurringAction;
        if (!pending) {
            this.onClose();
            return;
        }

        const occurrenceStart = this.eventPendingStart;
        this.eventPendingAction = undefined;
        this.eventPendingStart = undefined;
        this.recurringAction = 'delete';

        if (action === 'convert') {
            void this.convertEvent(pending, scope, occurrenceStart);
            return;
        }

        void store.mutate('Failed to delete event', () =>
            store.deleteEvent(pending.id, scope, occurrenceStart),
        );
        this.onClose();
    };

    handleRecurringCancel = (): void => {
        this.recurringModalOpen = false;
        this.recurringAction = 'delete';
        this.eventPendingAction = undefined;
        this.eventPendingStart = undefined;
    };
}

export function createInspectorState(
    getTarget: () => InspectorTarget | undefined,
    onClose: () => void,
    onConverted?: (task: AppTask) => void,
): InspectorPaneState {
    return new InspectorPaneState(getTarget, onClose, onConverted);
}
