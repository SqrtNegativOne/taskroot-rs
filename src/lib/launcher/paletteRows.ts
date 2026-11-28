import {
    taskDateOccursOnDay,
    type AppTask,
    type EventInstance,
    type TaskDateInstance,
    type TaskDateKind,
} from '../domain';
import { Routes } from '../routes';
import { formatHhMm, minutesSinceMidnight } from '../time';
import { formatQuickCaptureDue, type QuickCapturePreview } from './quickCapture';

export interface TodayTask {
    readonly taskId: string;
    readonly title: string;
    readonly kind: TaskDateKind;
}

export type PaletteRow =
    | { readonly key: 'create'; readonly kind: 'create' }
    | { readonly key: string; readonly kind: 'event'; readonly event: EventInstance }
    | { readonly key: string; readonly kind: 'task'; readonly task: TodayTask };

export const ROUTE_COMMANDS: Readonly<Record<string, string>> = {
    home: Routes.HOME,
    plan: Routes.HOME,
    do: Routes.DO,
    tags: Routes.TAGS,
    settings: Routes.SETTINGS,
};

export function matchRouteCommand(text: string): string | undefined {
    return ROUTE_COMMANDS[text.trim().toLowerCase()];
}

/**
 * Projects `queryTaskDates` markers onto open tasks occurring on `today`'s local
 * calendar day. Deduplicates tasks that have both a `start` and `due` marker
 * today, giving precedence to `due`.
 */
export function projectTodayTasks(
    markers: readonly TaskDateInstance[],
    today: Date,
): TodayTask[] {
    const byTaskId = new Map<string, TodayTask>();

    for (const marker of markers) {
        if (marker.status === 'done' || !taskDateOccursOnDay(marker, today)) continue;
        const existing = byTaskId.get(marker.taskId);
        if (existing === undefined) {
            byTaskId.set(marker.taskId, {
                taskId: marker.taskId,
                title: marker.title,
                kind: marker.kind,
            });
        } else if (marker.kind === 'due' && existing.kind !== 'due') {
            byTaskId.set(marker.taskId, { ...existing, kind: 'due' });
        }
    }

    return Array.from(byTaskId.values());
}

export function buildPaletteRows(
    text: string,
    todayEvents: readonly EventInstance[],
    todayTasks: readonly TodayTask[],
): PaletteRow[] {
    const value = text.trim();
    const typed = value !== '';
    const needle = value.toLowerCase();
    const result: PaletteRow[] = [];

    if (typed) result.push({ key: 'create', kind: 'create' });

    for (const event of todayEvents) {
        if (!typed || event.title.toLowerCase().includes(needle)) {
            result.push({ key: `event:${event.occurrenceKey}`, kind: 'event', event });
        }
    }

    for (const task of todayTasks) {
        if (!typed || task.title.toLowerCase().includes(needle)) {
            result.push({ key: `task:${task.taskId}`, kind: 'task', task });
        }
    }

    return result;
}

export function formatPreviewParts(value: QuickCapturePreview, today: Date): string[] {
    const parts: string[] = [];
    if (value.due !== null) {
        parts.push(formatQuickCaptureDue(value.due, value.hasTime, today));
    }
    if (value.tags.length > 0) parts.push(value.tags.map((tag) => `#${tag}`).join(' '));
    if (value.priority !== null) parts.push(`P${value.priority}`);
    if (value.estimateMinutes !== null) parts.push(`${value.estimateMinutes} min`);
    if (value.list !== null) parts.push(value.list.summary);
    return parts;
}

export function formatEventDetail(event: EventInstance): string {
    if (event.timing.kind === 'allDay') return 'All day';
    const start = new Date(event.timing.start);
    const end = new Date(event.timing.end);
    return `${formatHhMm(minutesSinceMidnight(start))}–${formatHhMm(minutesSinceMidnight(end))}`;
}

export function formatTaskDetail(task: TodayTask): string {
    return task.kind === 'due' ? 'Due today' : 'Planned today';
}

export function buildTaskFromQuickCapture(
    preview: QuickCapturePreview,
    timeZone: string,
): AppTask {
    return {
        id: crypto.randomUUID(),
        title: preview.title,
        status: 'todo',
        priority: preview.priority ?? undefined,
        tags: preview.tags.map((name) => ({ id: crypto.randomUUID(), name })),
        est: preview.estimateMinutes ?? undefined,
        due: preview.due ?? undefined,
        timezone: preview.hasTime ? timeZone : undefined,
        remoteCollectionId: preview.listQuery !== null ? preview.list?.id : undefined,
    };
}
