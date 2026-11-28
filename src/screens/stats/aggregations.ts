import type { AppEvent, AppTask, StopwatchHistoryEntry, Tag, TaskPriority } from '../../lib/domain';
import { ymd } from '../../lib/time';
import type { TagStat, TaskTimeStat } from './types';

export interface HistoryAggregation {
    workMs: number;
    breakMs: number;
    sessionCount: number;
    workMsByTask: Map<string, number>;
    workMsByDay: Map<string, number>;
    workMsByHour: number[];
    activeDays: Set<string>;
}

export interface TaskAggregation {
    doneTasks: number;
    openTasks: number;
    nextUpTasks: number;
    cancelledTasks: number;
    totalTaskTimeSpentSeconds: number;
    completedByDay: Map<string, number>;
    completedByHour: number[];
    trackedByTask: Map<string, number>;
    taskTimeCandidates: TaskTimeStat[];
    priorityTotals: Map<TaskPriority, number>;
    priorityDone: Map<TaskPriority, number>;
    activeDays: Set<string>;
}

const TOP_TAGS_LIMIT = 8;

function emptyHourBuckets(): number[] {
    return Array.from({ length: 24 }, () => 0);
}

function parseTimestamp(value: string | null | undefined): Date | null {
    if (!value) return null;
    const parsed = new Date(value);
    return Number.isNaN(parsed.getTime()) ? null : parsed;
}

function parseEpochMs(value: number): Date | null {
    if (!Number.isFinite(value)) return null;
    const parsed = new Date(value);
    return Number.isNaN(parsed.getTime()) ? null : parsed;
}

function incrementCount(map: Map<string, number>, key: string, amount: number): void {
    map.set(key, (map.get(key) ?? 0) + amount);
}

function incrementPriority(map: Map<TaskPriority, number>, key: TaskPriority): void {
    map.set(key, (map.get(key) ?? 0) + 1);
}

function positiveSeconds(value: number | null | undefined): number {
    if (value == null || !Number.isFinite(value) || value <= 0) return 0;
    return Math.round(value);
}

function positiveEstMinutes(value: number | null | undefined): number | null {
    if (value == null || !Number.isFinite(value) || value <= 0) return null;
    return value;
}

/** Aggregate stopwatch work/break durations, task time, daily/hourly buckets, and active days. */
export function aggregateHistory(history: readonly StopwatchHistoryEntry[]): HistoryAggregation {
    let workMs = 0;
    let breakMs = 0;
    let sessionCount = 0;
    const workMsByTask = new Map<string, number>();
    const workMsByDay = new Map<string, number>();
    const workMsByHour = emptyHourBuckets();
    const activeDays = new Set<string>();

    for (const entry of history) {
        const ms = Number.isFinite(entry.durationMs) && entry.durationMs > 0 ? entry.durationMs : 0;
        if (entry.kind === 'break') {
            breakMs += ms;
            continue;
        }
        workMs += ms;
        sessionCount += 1;
        if (entry.taskId) {
            incrementCount(workMsByTask, entry.taskId, ms);
        }
        const started = parseEpochMs(entry.startedAt);
        if (started) {
            const day = ymd(started);
            activeDays.add(day);
            incrementCount(workMsByDay, day, ms);
            workMsByHour[started.getHours()] += ms;
        }
    }

    return { workMs, breakMs, sessionCount, workMsByTask, workMsByDay, workMsByHour, activeDays };
}

function recordCompletedTask(task: AppTask, acc: TaskAggregation): void {
    const completedAt = parseTimestamp(task.completedAt);
    if (!completedAt) return;
    const day = ymd(completedAt);
    acc.activeDays.add(day);
    incrementCount(acc.completedByDay, day, 1);
    acc.completedByHour[completedAt.getHours()] += 1;
}

function accumulateTask(
    task: AppTask,
    workMsByTask: ReadonlyMap<string, number>,
    acc: TaskAggregation,
): void {
    const status = task.status ?? 'todo';
    if (status === 'done') acc.doneTasks += 1;
    else if (status === 'next-up') acc.nextUpTasks += 1;
    else if (status === 'cancelled') acc.cancelledTasks += 1;
    else acc.openTasks += 1;

    const priority: TaskPriority = task.priority ?? 0;
    incrementPriority(acc.priorityTotals, priority);
    if (status === 'done') incrementPriority(acc.priorityDone, priority);

    const taskSecs = positiveSeconds(task.timeSpent);
    acc.totalTaskTimeSpentSeconds += taskSecs;
    const histTaskSecs = Math.round((workMsByTask.get(task.id) ?? 0) / 1000);
    const combinedSecs = Math.max(taskSecs, histTaskSecs);
    acc.trackedByTask.set(task.id, combinedSecs);
    if (combinedSecs > 0) {
        acc.taskTimeCandidates.push({
            id: task.id,
            title: task.title,
            status,
            trackedSeconds: combinedSecs,
            estMinutes: positiveEstMinutes(task.est),
        });
    }
    recordCompletedTask(task, acc);
}

/** Aggregate task statuses, priorities, completions, and combined tracked durations. */
export function aggregateTasks(
    tasks: readonly AppTask[],
    workMsByTask: ReadonlyMap<string, number>,
): TaskAggregation {
    const acc: TaskAggregation = {
        doneTasks: 0,
        openTasks: 0,
        nextUpTasks: 0,
        cancelledTasks: 0,
        totalTaskTimeSpentSeconds: 0,
        completedByDay: new Map(),
        completedByHour: emptyHourBuckets(),
        trackedByTask: new Map(),
        taskTimeCandidates: [],
        priorityTotals: new Map(),
        priorityDone: new Map(),
        activeDays: new Set(),
    };

    for (const task of tasks) {
        accumulateTask(task, workMsByTask, acc);
    }

    return acc;
}

function getOrCreateTag(map: Map<string, TagStat>, name: string, color: string | null | undefined): TagStat {
    const existing = map.get(name);
    if (existing) {
        if (!existing.color && color) existing.color = color;
        return existing;
    }
    const created: TagStat = {
        name,
        color: color ?? null,
        taskCount: 0,
        eventCount: 0,
        totalCount: 0,
        doneCount: 0,
        trackedSeconds: 0,
    };
    map.set(name, created);
    return created;
}

function uniqueTags(tags: readonly Tag[] | null | undefined): Tag[] {
    if (!tags || tags.length === 0) return [];
    const seen = new Set<string>();
    const result: Tag[] = [];
    for (const tag of tags) {
        if (seen.has(tag.name)) continue;
        seen.add(tag.name);
        result.push(tag);
    }
    return result;
}

function compareTags(a: TagStat, b: TagStat): number {
    if (b.totalCount !== a.totalCount) return b.totalCount - a.totalCount;
    if (b.trackedSeconds !== a.trackedSeconds) return b.trackedSeconds - a.trackedSeconds;
    return a.name.localeCompare(b.name);
}

/** Aggregate top tags across tasks and events with completion and tracked time totals. */
export function aggregateTopTags(
    tasks: readonly AppTask[],
    events: readonly AppEvent[],
    trackedByTask: ReadonlyMap<string, number>,
): TagStat[] {
    const map = new Map<string, TagStat>();

    for (const task of tasks) {
        const taskTracked = trackedByTask.get(task.id) ?? 0;
        const isDone = task.status === 'done';
        for (const tag of uniqueTags(task.tags)) {
            const stat = getOrCreateTag(map, tag.name, tag.color);
            stat.taskCount += 1;
            stat.totalCount += 1;
            if (isDone) stat.doneCount += 1;
            stat.trackedSeconds += taskTracked;
        }
    }

    for (const event of events) {
        for (const tag of uniqueTags(event.tags)) {
            const stat = getOrCreateTag(map, tag.name, tag.color);
            stat.eventCount += 1;
            stat.totalCount += 1;
        }
    }

    return [...map.values()].sort(compareTags).slice(0, TOP_TAGS_LIMIT);
}
