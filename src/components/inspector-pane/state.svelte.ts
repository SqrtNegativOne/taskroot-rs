import { untrack } from 'svelte';
import type {
    AppCalendar,
    AppEvent,
    AppTask,
    EventPatch,
    EventScope,
    TaskPatch,
} from '../../lib/domain';
import {
    DEFAULT_TASK_PRIORITY,
    isCalendarReadOnly,
    isEventReadOnly,
    isRecurring,
    resolveDefaultCalendar,
} from '../../lib/domain';
import { allEvents, allTasks } from '../../lib/queries';
import { commands, useAutoQuery, type AutoQueryResult } from '../../lib/safeInvoke.svelte';
import { store } from '../../lib/store.svelte';
import { notifyError } from '../../lib/toast';
import { eventStartDate } from './eventDates';
import { buildTaskQuickEditPatches, type TaskTitleQuickEdit } from './quickEdit';

export type InspectorTarget =
    | { type: 'task'; id: string; draft?: AppTask }
    | { type: 'event'; id: string; occurrenceStart?: string; draft?: AppEvent };

/** Reactive inspector state: queries, the selected item, mutations and the recurring-delete scope prompt. */
export class InspectorPaneState {
    recurringModalOpen = $state(false);
    recurringAction = $state<'delete' | 'convert'>('delete');
    private eventPendingAction = $state<AppEvent | undefined>(undefined);
    private eventPendingStart = $state<string | undefined>(undefined);

    private readonly tasksQuery = useAutoQuery(commands.queryTasks, allTasks);
    private readonly eventsQuery = useAutoQuery(commands.queryEvents, allEvents);
    private readonly tagsQuery = useAutoQuery(commands.listTags);
    private readonly calendarsQuery: AutoQueryResult<AppCalendar[]> | undefined;

    constructor(
        private readonly getTarget: () => InspectorTarget | undefined,
        private readonly onClose: () => void,
        private readonly onConverted: (task: AppTask) => void = onClose,
        private readonly getCalendars?: () => AppCalendar[] | undefined,
    ) {
        this.calendarsQuery = getCalendars === undefined || untrack(() => getCalendars() === undefined)
            ? useAutoQuery(commands.getActiveCalendars)
            : undefined;
    }

    get tasks(): AppTask[] {
        return this.tasksQuery.data ?? [];
    }

    get events(): AppEvent[] {
        return this.eventsQuery.data ?? [];
    }

    get availableTags(): string[] {
        return this.tagsQuery.data ?? [];
    }

    get calendars(): AppCalendar[] {
        return this.getCalendars?.() ?? this.calendarsQuery?.data ?? [];
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
        const calendars = this.calendars;
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

    updateTask = (id: string, patches: TaskPatch[]): void => {
        void store.mutate('Failed to update task', () => store.updateTask(id, patches));
    };

    updateEvent = (id: string, patches: EventPatch[]): void => {
        void store.mutate('Failed to update event', () => store.updateEvent(id, patches));
    };

    handleTitleChange = (value: string): void => {
        const task = this.currentTask;
        if (task) {
            this.updateTask(task.id, [{ field: 'title', value }]);
            return;
        }
        const event = this.currentEvent;
        if (event) this.updateEvent(event.id, [{ field: 'title', value }]);
    };

    handleTaskQuickEdit = (edit: TaskTitleQuickEdit): void => {
        const task = this.currentTask;
        if (!task) return;
        const patches = buildTaskQuickEditPatches(task, edit);
        if (patches.length === 0) return;
        this.updateTask(task.id, patches);
    };

    handleDescriptionChange = (value: string): void => {
        const task = this.currentTask;
        if (task) {
            this.updateTask(task.id, [{ field: 'notes', value: value || null }]);
            return;
        }
        const event = this.currentEvent;
        if (event) this.updateEvent(event.id, [{ field: 'description', value: value || null }]);
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
        const taskList = resolveDefaultCalendar(
            this.calendars,
            'VTODO',
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
    getCalendars?: () => AppCalendar[] | undefined,
): InspectorPaneState {
    return new InspectorPaneState(getTarget, onClose, onConverted, getCalendars);
}
