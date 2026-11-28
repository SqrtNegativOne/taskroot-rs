import type { AppTask, StopwatchHistoryEntry } from '../../lib/domain';
import { addDays, ymd } from '../../lib/time';
import {
    boundedLayoutWindow,
    clamp,
    computeStartHour,
    formatWrapDuration,
    layoutTimelineItems,
    MINUTES_IN_HOUR,
    MINUTES_PER_DAY,
    MS_PER_SECOND,
    type UnlaidWrapItem,
    type WrapTimelineItem,
    type WrapTimelineItemKind,
} from './wrapLayout';

export { formatWrapDuration, type WrapTimelineItem, type WrapTimelineItemKind };

const MIN_WORK_VISUAL_MINS = 20;
const MIN_COMPLETED_VISUAL_MINS = 20;
const DEFAULT_COMPLETED_SPAN_MINS = 25;
const MAX_COMPLETED_SPAN_MINS = 240;
const COMPLETION_PIN_VISUAL_MINS = 16;

export interface WrapDaySummary {
    wrapDate: Date;
    wrapYmd: string;
    startHour: number;
    completedTasks: AppTask[];
    completedCount: number;
    totalTrackedMs: number;
    workSessionCount: number;
    items: WrapTimelineItem[];
}

function exactMinutesSinceMidnight(date: Date): number {
    return date.getHours() * MINUTES_IN_HOUR + date.getMinutes() + date.getSeconds() / 60;
}

function parseTimestamp(value: string | null | undefined): Date | null {
    if (!value) return null;
    const parsed = new Date(value);
    return Number.isNaN(parsed.getTime()) ? null : parsed;
}

function isSameLocalDay(candidate: Date, dayStartMs: number, dayEndMs: number): boolean {
    const ms = candidate.getTime();
    return ms >= dayStartMs && ms < dayEndMs;
}

function completedBlockSpanMins(task: AppTask): number {
    if (task.timeSpent != null && task.timeSpent > 0) {
        return clamp(Math.round(task.timeSpent / 60), 15, MAX_COMPLETED_SPAN_MINS);
    }
    if (task.est != null && task.est > 0) {
        return clamp(task.est, 15, MAX_COMPLETED_SPAN_MINS);
    }
    return DEFAULT_COMPLETED_SPAN_MINS;
}

function toWorkItem(
    entry: StopwatchHistoryEntry,
    index: number,
    taskById: ReadonlyMap<string, AppTask>,
    dayStartMs: number,
    dayEndMs: number,
): UnlaidWrapItem | null {
    if (entry.kind !== 'work') return null;
    if (!Number.isFinite(entry.startedAt) || !Number.isFinite(entry.endedAt)) return null;
    if (entry.endedAt <= entry.startedAt) return null;
    if (entry.startedAt >= dayEndMs || entry.endedAt <= dayStartMs) return null;

    const clippedStartMs = Math.max(entry.startedAt, dayStartMs);
    const clippedEndMs = Math.min(entry.endedAt, dayEndMs);
    const durationMs = clippedEndMs - clippedStartMs;
    if (durationMs <= 0) return null;

    const startMins = clippedStartMs <= dayStartMs
        ? 0
        : exactMinutesSinceMidnight(new Date(clippedStartMs));
    const endMins = clippedEndMs >= dayEndMs
        ? MINUTES_PER_DAY
        : Math.max(startMins, exactMinutesSinceMidnight(new Date(clippedEndMs)));
    const { layoutStartMins, layoutEndMins } = boundedLayoutWindow(
        startMins,
        endMins,
        MIN_WORK_VISUAL_MINS,
    );

    const taskId = entry.taskId ?? undefined;
    const linkedTask = taskId ? taskById.get(taskId) : undefined;
    const title = linkedTask?.title.trim() || entry.label.trim() || 'Focus session';

    return {
        id: `work-${index}-${entry.startedAt}`,
        kind: 'work',
        taskId,
        title,
        startMins,
        endMins,
        layoutStartMins,
        layoutEndMins,
        durationMs,
        isDone: linkedTask?.status === 'done',
    };
}

function buildWorkItems(
    history: readonly StopwatchHistoryEntry[],
    taskById: ReadonlyMap<string, AppTask>,
    dayStartMs: number,
    dayEndMs: number,
): { items: UnlaidWrapItem[]; tasksWithWork: Set<string>; totalTrackedMs: number } {
    const items: UnlaidWrapItem[] = [];
    const tasksWithWork = new Set<string>();
    let totalTrackedMs = 0;

    for (let i = 0; i < history.length; i++) {
        const item = toWorkItem(history[i]!, i, taskById, dayStartMs, dayEndMs);
        if (!item) continue;
        if (item.taskId) tasksWithWork.add(item.taskId);
        totalTrackedMs += item.durationMs ?? 0;
        items.push(item);
    }

    return { items, tasksWithWork, totalTrackedMs };
}

function buildCompletedItems(
    completedTasks: readonly AppTask[],
    tasksWithWork: ReadonlySet<string>,
): UnlaidWrapItem[] {
    const items: UnlaidWrapItem[] = [];

    for (const task of completedTasks) {
        const completedDate = parseTimestamp(task.completedAt);
        if (!completedDate) continue;
        const completedMins = clamp(exactMinutesSinceMidnight(completedDate), 0, MINUTES_PER_DAY - 0.01);

        if (tasksWithWork.has(task.id)) {
            const { layoutStartMins, layoutEndMins } = boundedLayoutWindow(
                completedMins,
                completedMins,
                COMPLETION_PIN_VISUAL_MINS,
            );
            items.push({
                id: `pin-${task.id}`,
                kind: 'completion-pin',
                taskId: task.id,
                title: task.title,
                startMins: completedMins,
                endMins: completedMins,
                layoutStartMins,
                layoutEndMins,
                durationMs: task.timeSpent && task.timeSpent > 0 ? task.timeSpent * MS_PER_SECOND : undefined,
                completedAtMins: completedMins,
                isDone: true,
            });
            continue;
        }

        const spanMins = completedBlockSpanMins(task);
        const endMins = completedMins;
        const startMins = Math.max(0, endMins - spanMins);
        const layoutEndMins = Math.min(MINUTES_PER_DAY, Math.max(endMins, startMins + MIN_COMPLETED_VISUAL_MINS));
        const layoutStartMins = Math.min(startMins, Math.max(0, layoutEndMins - MIN_COMPLETED_VISUAL_MINS));

        items.push({
            id: `done-${task.id}`,
            kind: 'completed',
            taskId: task.id,
            title: task.title,
            startMins,
            endMins,
            layoutStartMins,
            layoutEndMins,
            durationMs: task.timeSpent && task.timeSpent > 0 ? task.timeSpent * MS_PER_SECOND : undefined,
            completedAtMins: completedMins,
            isDone: true,
        });
    }

    return items;
}

/**
 * Builds the timeline items and summary metrics for one evening wrap date.
 */
export function buildWrapDaySummary(
    tasks: readonly AppTask[],
    history: readonly StopwatchHistoryEntry[],
    wrapDate: Date,
    earliestWakeUpTime?: number | null,
): WrapDaySummary {
    const dayStart = new Date(wrapDate.getFullYear(), wrapDate.getMonth(), wrapDate.getDate());
    const dayEnd = addDays(dayStart, 1);
    const dayStartMs = dayStart.getTime();
    const dayEndMs = dayEnd.getTime();

    const taskById = new Map<string, AppTask>();
    const completedTasks: AppTask[] = [];
    for (const task of tasks) {
        taskById.set(task.id, task);
        if (task.status !== 'done') continue;
        const completedDate = parseTimestamp(task.completedAt);
        if (completedDate && isSameLocalDay(completedDate, dayStartMs, dayEndMs)) {
            completedTasks.push(task);
        }
    }

    completedTasks.sort((a, b) => {
        const aMs = parseTimestamp(a.completedAt)?.getTime() ?? 0;
        const bMs = parseTimestamp(b.completedAt)?.getTime() ?? 0;
        return aMs - bMs || a.title.localeCompare(b.title);
    });

    const work = buildWorkItems(history, taskById, dayStartMs, dayEndMs);
    const completed = buildCompletedItems(completedTasks, work.tasksWithWork);
    const rawItems = [...work.items, ...completed];

    return {
        wrapDate: dayStart,
        wrapYmd: ymd(dayStart),
        startHour: computeStartHour(rawItems, earliestWakeUpTime),
        completedTasks,
        completedCount: completedTasks.length,
        totalTrackedMs: work.totalTrackedMs,
        workSessionCount: work.items.length,
        items: layoutTimelineItems(rawItems),
    };
}
