import type { TaskPriority } from '../../lib/domain';
import { addDays, dayDiff, parseLocalYmd, ymd } from '../../lib/time';
import { aggregateHistory, aggregateTasks, aggregateTopTags } from './aggregations';
import type {
    DailyActivityPoint,
    HourlyActivityPoint,
    PriorityStat,
    StatsInput,
    StatsSummary,
    TaskTimeStat,
} from './types';

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'] as const;
const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'] as const;

const PRIORITY_META: ReadonlyArray<{ priority: TaskPriority; label: string; cssVar: string }> = [
    { priority: 4, label: 'Urgent', cssVar: '--priority-4' },
    { priority: 3, label: 'High', cssVar: '--priority-3' },
    { priority: 2, label: 'Medium', cssVar: '--priority-2' },
    { priority: 1, label: 'Low', cssVar: '--priority-1' },
    { priority: 0, label: 'None', cssVar: '--priority-0' },
];

const WINDOW_DAYS = 14;
const TOP_TASKS_LIMIT = 5;

/** Format a whole-second duration as `"14h 25m"`, `"42m"`, `"<1m"`, or `"0m"`. */
export function formatDuration(seconds: number): string {
    if (!Number.isFinite(seconds) || seconds <= 0) return '0m';
    if (seconds < 60) return '<1m';
    const totalMinutes = Math.floor(seconds / 60);
    const hours = Math.floor(totalMinutes / 60);
    const minutes = totalMinutes % 60;
    if (hours === 0) return `${minutes}m`;
    if (minutes === 0) return `${hours}h`;
    return `${hours}h ${minutes}m`;
}

function localDayStart(date: Date): Date {
    return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function pad2(value: number): string {
    return value.toString().padStart(2, '0');
}

function computeStreaks(activeDays: Set<string>, today: Date): { current: number; longest: number } {
    const todayStr = ymd(today);
    const yesterday = addDays(today, -1);
    const startCursor = activeDays.has(todayStr)
        ? today
        : activeDays.has(ymd(yesterday))
            ? yesterday
            : null;

    let current = 0;
    let cursor = startCursor;
    while (cursor && activeDays.has(ymd(cursor))) {
        current += 1;
        cursor = addDays(cursor, -1);
    }

    const sorted = [...activeDays].sort();
    let longest = 0;
    let run = 0;
    let prevDate: Date | null = null;
    for (const dayStr of sorted) {
        const date = parseLocalYmd(dayStr);
        run = prevDate && dayDiff(date, prevDate) === 1 ? run + 1 : 1;
        if (run > longest) longest = run;
        prevDate = date;
    }

    return { current, longest };
}

function buildDailyActivity(
    today: Date,
    completedByDay: ReadonlyMap<string, number>,
    workMsByDay: ReadonlyMap<string, number>,
): DailyActivityPoint[] {
    const points: DailyActivityPoint[] = [];
    for (let offset = -(WINDOW_DAYS - 1); offset <= 0; offset += 1) {
        const day = addDays(today, offset);
        const date = ymd(day);
        const workMs = workMsByDay.get(date) ?? 0;
        points.push({
            date,
            label: `${MONTHS[day.getMonth()]} ${day.getDate()}`,
            shortLabel: `${day.getDate()}`,
            weekday: WEEKDAYS[day.getDay()],
            completedCount: completedByDay.get(date) ?? 0,
            trackedMinutes: Math.round(workMs / 60_000),
        });
    }
    return points;
}

function buildHourlyActivity(
    completedByHour: readonly number[],
    workMsByHour: readonly number[],
): { buckets: HourlyActivityPoint[]; peakHour: number | null; peakHourLabel: string } {
    const buckets: HourlyActivityPoint[] = [];
    let peakHour: number | null = null;
    let bestScore = 0;

    for (let hour = 0; hour < 24; hour += 1) {
        const completedCount = completedByHour[hour] ?? 0;
        const trackedMinutes = Math.round((workMsByHour[hour] ?? 0) / 60_000);
        const score = trackedMinutes + completedCount * 15;
        buckets.push({ hour, label: pad2(hour), completedCount, trackedMinutes, score });
        if (score > bestScore) {
            bestScore = score;
            peakHour = hour;
        }
    }

    const peakHourLabel = peakHour === null
        ? 'No activity yet'
        : `${pad2(peakHour)}:00 – ${pad2((peakHour + 1) % 24)}:00`;

    return { buckets, peakHour, peakHourLabel };
}

function compareTasksByTime(a: TaskTimeStat, b: TaskTimeStat): number {
    if (b.trackedSeconds !== a.trackedSeconds) return b.trackedSeconds - a.trackedSeconds;
    return a.title.localeCompare(b.title);
}

function buildPriorityBreakdown(
    totals: ReadonlyMap<TaskPriority, number>,
    doneCounts: ReadonlyMap<TaskPriority, number>,
): PriorityStat[] {
    const rows: PriorityStat[] = [];
    for (const meta of PRIORITY_META) {
        const total = totals.get(meta.priority) ?? 0;
        const done = doneCounts.get(meta.priority) ?? 0;
        rows.push({
            priority: meta.priority,
            label: meta.label,
            cssVar: meta.cssVar,
            total,
            done,
            completionPct: total > 0 ? Math.round((done / total) * 100) : 0,
        });
    }
    return rows;
}

/** Compute all-time ("forever") productivity and completion statistics. */
export function computeStats(input: StatsInput): StatsSummary {
    const today = localDayStart(input.now ?? new Date());
    const historyAgg = aggregateHistory(input.history);
    const taskAgg = aggregateTasks(input.tasks, historyAgg.workMsByTask);

    const activeDays = new Set<string>(historyAgg.activeDays);
    for (const day of taskAgg.activeDays) {
        activeDays.add(day);
    }

    let scheduledTaskBlocks = 0;
    for (const event of input.events) {
        if (event.taskId) scheduledTaskBlocks += 1;
    }

    const totalWorkSeconds = Math.round(historyAgg.workMs / 1000);
    const totalBreakSeconds = Math.round(historyAgg.breakMs / 1000);
    const totalTrackedSeconds = Math.max(totalWorkSeconds, taskAgg.totalTaskTimeSpentSeconds);
    const activeTimeDenominator = totalTrackedSeconds + totalBreakSeconds;
    const focusRatioPct = activeTimeDenominator > 0
        ? Math.round((totalTrackedSeconds / activeTimeDenominator) * 100)
        : 0;
    const avgSessionMinutes = historyAgg.sessionCount > 0
        ? Math.round(totalWorkSeconds / 60 / historyAgg.sessionCount)
        : 0;

    const totalTasks = input.tasks.length;
    const completionRatePct = totalTasks > 0 ? Math.round((taskAgg.doneTasks / totalTasks) * 100) : 0;
    const streaks = computeStreaks(activeDays, today);
    const dailyActivity = buildDailyActivity(today, taskAgg.completedByDay, historyAgg.workMsByDay);
    const hourly = buildHourlyActivity(taskAgg.completedByHour, historyAgg.workMsByHour);
    const topTags = aggregateTopTags(input.tasks, input.events, taskAgg.trackedByTask);
    const topTasksByTime = taskAgg.taskTimeCandidates.sort(compareTasksByTime).slice(0, TOP_TASKS_LIMIT);
    const priorityBreakdown = buildPriorityBreakdown(taskAgg.priorityTotals, taskAgg.priorityDone);

    return {
        totalTrackedSeconds,
        totalWorkSeconds,
        totalTaskTimeSpentSeconds: taskAgg.totalTaskTimeSpentSeconds,
        totalBreakSeconds,
        focusRatioPct,
        sessionCount: historyAgg.sessionCount,
        avgSessionMinutes,
        totalTasks,
        doneTasks: taskAgg.doneTasks,
        openTasks: taskAgg.openTasks,
        nextUpTasks: taskAgg.nextUpTasks,
        cancelledTasks: taskAgg.cancelledTasks,
        completionRatePct,
        totalEvents: input.events.length,
        scheduledTaskBlocks,
        currentStreakDays: streaks.current,
        longestStreakDays: streaks.longest,
        activeDaysTotal: activeDays.size,
        dailyActivity,
        hourlyActivity: hourly.buckets,
        peakHour: hourly.peakHour,
        peakHourLabel: hourly.peakHourLabel,
        topTags,
        totalKnownTags: input.tags?.length ?? topTags.length,
        topTasksByTime,
        priorityBreakdown,
    };
}
