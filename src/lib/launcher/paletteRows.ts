import type { AppTask, EventInstance } from '../domain';
import { Routes } from '../routes';
import { formatHhMm, minutesSinceMidnight } from '../time';
import { formatQuickCaptureDue, type QuickCapturePreview } from './quickCapture';

export type PaletteRow =
    | { readonly key: 'create'; readonly kind: 'create' }
    | { readonly key: string; readonly kind: 'event'; readonly event: EventInstance }
    | { readonly key: string; readonly kind: 'task'; readonly task: AppTask };

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

export function filterTodayTasks(tasks: readonly AppTask[], todayIso: string): AppTask[] {
    return tasks.filter(
        (task) => task.start?.startsWith(todayIso) === true || task.due?.startsWith(todayIso) === true,
    );
}

export function buildPaletteRows(
    text: string,
    todayEvents: readonly EventInstance[],
    todayTasks: readonly AppTask[],
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
            result.push({ key: `task:${task.id}`, kind: 'task', task });
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

export function formatTaskDetail(task: AppTask, todayIso: string): string {
    return task.due?.startsWith(todayIso) === true ? 'Due today' : 'Planned today';
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
