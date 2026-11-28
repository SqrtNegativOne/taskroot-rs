import type { AppEvent, AppTask, AppTaskStatus, StopwatchHistoryEntry, TaskPriority } from '../../lib/domain';

export interface StatsInput {
    tasks: readonly AppTask[];
    events: readonly AppEvent[];
    history: readonly StopwatchHistoryEntry[];
    tags?: readonly string[];
    now?: Date;
}

export interface DailyActivityPoint {
    date: string;
    label: string;
    shortLabel: string;
    weekday: string;
    completedCount: number;
    trackedMinutes: number;
}

export interface HourlyActivityPoint {
    hour: number;
    label: string;
    completedCount: number;
    trackedMinutes: number;
    score: number;
}

export interface TagStat {
    name: string;
    color: string | null;
    taskCount: number;
    eventCount: number;
    totalCount: number;
    doneCount: number;
    trackedSeconds: number;
}

export interface TaskTimeStat {
    id: string;
    title: string;
    status: AppTaskStatus;
    trackedSeconds: number;
    estMinutes: number | null;
}

export interface PriorityStat {
    priority: TaskPriority;
    label: string;
    cssVar: string;
    total: number;
    done: number;
    completionPct: number;
}

export interface StatsSummary {
    totalTrackedSeconds: number;
    totalWorkSeconds: number;
    totalTaskTimeSpentSeconds: number;
    totalBreakSeconds: number;
    focusRatioPct: number;
    sessionCount: number;
    avgSessionMinutes: number;
    totalTasks: number;
    doneTasks: number;
    openTasks: number;
    nextUpTasks: number;
    cancelledTasks: number;
    completionRatePct: number;
    totalEvents: number;
    scheduledTaskBlocks: number;
    currentStreakDays: number;
    longestStreakDays: number;
    activeDaysTotal: number;
    dailyActivity: DailyActivityPoint[];
    hourlyActivity: HourlyActivityPoint[];
    peakHour: number | null;
    peakHourLabel: string;
    topTags: TagStat[];
    totalKnownTags: number;
    topTasksByTime: TaskTimeStat[];
    priorityBreakdown: PriorityStat[];
}
