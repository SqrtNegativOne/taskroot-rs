import type { AppCalendar } from '../bindings/AppCalendar.generated';
import type { AppEvent } from '../bindings/AppEvent.generated';
import type { AppTask } from '../bindings/AppTask.generated';

export type HydratedEvent = AppEvent & {
    task?: AppTask;
    color?: string;
    category?: string;
    isInstance?: boolean;
    baseEventId?: string;
};

export type CalendarLike = AppCalendar | {
    readonly id: string;
    readonly summary?: string;
    readonly color?: string;
    readonly backgroundColor?: string;
    readonly isPrimary?: boolean;
    readonly primary?: boolean;
};

export function resolveEventCalendar<C extends CalendarLike>(
    ev: { readonly remoteCollectionId?: string },
    calendars: readonly C[] = [],
): C | undefined {
    if (calendars.length === 0) {
        return undefined;
    }

    const isPrimaryCal = (c: C): boolean => Boolean(c.isPrimary ?? (c as { readonly primary?: boolean }).primary);
    const primaryCal = calendars.find(isPrimaryCal) ?? calendars[0];

    const collectionId = ev.remoteCollectionId;
    if (!collectionId || collectionId === 'primary') {
        return primaryCal;
    }

    return calendars.find((c) => c.id === collectionId) ?? primaryCal;
}

export function isEventAllDay(event: {
    readonly startTime: string;
    readonly endTime: string;
    readonly isAllDay?: boolean;
}): boolean {
    if (typeof event.isAllDay === 'boolean') {
        return event.isAllDay;
    }
    const YMD_LENGTH = 10;
    return (
        event.startTime.length === YMD_LENGTH &&
        event.endTime.length === YMD_LENGTH &&
        !event.startTime.includes('T') &&
        !event.startTime.includes(' ')
    );
}

function resolveEventTitle(ev: AppEvent, task: AppTask | undefined, taskId: string | undefined): string {
    if (task) {
        return task.title;
    }
    if (ev.title) {
        return ev.title;
    }
    if (taskId) {
        return 'Unknown Task';
    }
    return '';
}

function hydrateSingleEvent(
    ev: AppEvent,
    taskMap: Map<string, AppTask>,
    calendars: readonly CalendarLike[],
): HydratedEvent {
    const cal = resolveEventCalendar(ev, calendars);
    const calColor = cal?.color ?? (cal as { readonly backgroundColor?: string } | undefined)?.backgroundColor;
    const color = calColor ?? ev.color;
    const category = cal?.summary;

    const taskId = ev.taskId ?? (ev as { readonly task_id?: string }).task_id;
    const task = taskId ? taskMap.get(taskId) : undefined;
    const title = resolveEventTitle(ev, task, taskId);

    const result: HydratedEvent = { ...ev, title };
    if (task !== undefined) {
        result.task = task;
    }
    if (color !== undefined) {
        result.color = color;
    }
    if (category !== undefined) {
        result.category = category;
    }
    return result;
}

export function hydrateEvents(
    events: readonly AppEvent[],
    tasks: readonly AppTask[] = [],
    calendars: readonly CalendarLike[] = [],
): HydratedEvent[] {
    const taskMap = new Map<string, AppTask>();
    for (const task of tasks) {
        taskMap.set(task.id, task);
    }

    return events.map((ev) => hydrateSingleEvent(ev, taskMap, calendars));
}
