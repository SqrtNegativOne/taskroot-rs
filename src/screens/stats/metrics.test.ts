import { describe, expect, it } from 'vitest';
import type { AppEvent, AppTask, StopwatchHistoryEntry } from '../../lib/domain';
import { computeStats, formatDuration } from './metrics';

function makeTask(overrides: Partial<AppTask> & { id: string; title: string }): AppTask {
    return {
        status: 'todo',
        priority: 2,
        ...overrides,
    };
}

function makeEvent(overrides: Partial<AppEvent> & { id: string; title: string }): AppEvent {
    return {
        startTime: '2026-11-27T09:00:00',
        endTime: '2026-11-27T10:00:00',
        ...overrides,
    };
}

function makeWorkEntry(
    startedAt: Date,
    durationMinutes: number,
    taskId: string | null = null,
): StopwatchHistoryEntry {
    const durationMs = durationMinutes * 60_000;
    const startMs = startedAt.getTime();
    return {
        kind: 'work',
        taskId,
        label: taskId ?? 'Focus',
        labelSource: taskId ? 'task' : 'custom',
        startedAt: startMs,
        endedAt: startMs + durationMs,
        durationMs,
    };
}

function makeBreakEntry(startedAt: Date, durationMinutes: number): StopwatchHistoryEntry {
    const durationMs = durationMinutes * 60_000;
    const startMs = startedAt.getTime();
    return {
        kind: 'break',
        taskId: null,
        label: 'Break',
        labelSource: 'unset',
        startedAt: startMs,
        endedAt: startMs + durationMs,
        durationMs,
    };
}

describe('formatDuration', () => {
    it('formats zero, sub-minute, minutes, whole hours, and hours+minutes', () => {
        expect(formatDuration(0)).toBe('0m');
        expect(formatDuration(-15)).toBe('0m');
        expect(formatDuration(Number.NaN)).toBe('0m');
        expect(formatDuration(45)).toBe('<1m');
        expect(formatDuration(42 * 60)).toBe('42m');
        expect(formatDuration(3 * 3600)).toBe('3h');
        expect(formatDuration(14 * 3600 + 25 * 60)).toBe('14h 25m');
    });
});

describe('computeStats', () => {
    it('returns safe zeroed defaults on empty inputs', () => {
        const now = new Date(2026, 10, 27, 15, 0, 0);
        const stats = computeStats({ tasks: [], events: [], history: [], tags: [], now });

        expect(stats.totalTrackedSeconds).toBe(0);
        expect(stats.focusRatioPct).toBe(0);
        expect(stats.sessionCount).toBe(0);
        expect(stats.avgSessionMinutes).toBe(0);
        expect(stats.totalTasks).toBe(0);
        expect(stats.completionRatePct).toBe(0);
        expect(stats.currentStreakDays).toBe(0);
        expect(stats.longestStreakDays).toBe(0);
        expect(stats.peakHour).toBeNull();
        expect(stats.peakHourLabel).toBe('No activity yet');
        expect(stats.dailyActivity).toHaveLength(14);
        expect(stats.hourlyActivity).toHaveLength(24);
        expect(stats.priorityBreakdown).toHaveLength(5);
        expect(stats.topTasksByTime).toEqual([]);
        expect(stats.topTags).toEqual([]);
    });

    it('continues current streak from yesterday when today has no activity yet', () => {
        const now = new Date(2026, 10, 27, 10, 0, 0);
        const tasks: AppTask[] = [
            makeTask({ id: 't1', title: 'Day -1', status: 'done', completedAt: new Date(2026, 10, 26, 14, 0).toISOString() }),
            makeTask({ id: 't2', title: 'Day -2', status: 'done', completedAt: new Date(2026, 10, 25, 16, 0).toISOString() }),
            makeTask({ id: 't3', title: 'Old run 1', status: 'done', completedAt: new Date(2026, 10, 10, 12, 0).toISOString() }),
            makeTask({ id: 't4', title: 'Old run 2', status: 'done', completedAt: new Date(2026, 10, 11, 12, 0).toISOString() }),
            makeTask({ id: 't5', title: 'Old run 3', status: 'done', completedAt: new Date(2026, 10, 12, 12, 0).toISOString() }),
            makeTask({ id: 't6', title: 'Old run 4', status: 'done', completedAt: new Date(2026, 10, 13, 12, 0).toISOString() }),
        ];
        const history: StopwatchHistoryEntry[] = [
            makeWorkEntry(new Date(2026, 10, 24, 9, 0), 30),
            makeBreakEntry(new Date(2026, 10, 27, 9, 30), 10),
        ];

        const stats = computeStats({ tasks, events: [], history, now });

        expect(stats.currentStreakDays).toBe(3);
        expect(stats.longestStreakDays).toBe(4);
        expect(stats.activeDaysTotal).toBe(7);
    });

    it('breaks current streak when neither today nor yesterday was active', () => {
        const now = new Date(2026, 10, 27, 10, 0, 0);
        const tasks: AppTask[] = [
            makeTask({ id: 't1', title: 'Two days ago', status: 'done', completedAt: new Date(2026, 10, 25, 11, 0).toISOString() }),
        ];

        const stats = computeStats({ tasks, events: [], history: [], now });
        expect(stats.currentStreakDays).toBe(0);
        expect(stats.longestStreakDays).toBe(1);
    });

    it('reconciles stopwatch work history with task.timeSpent without double-counting', () => {
        const now = new Date(2026, 10, 27, 18, 0, 0);
        const tasks: AppTask[] = [
            makeTask({ id: 't1', title: 'Shared task', status: 'done', timeSpent: 3600, est: 90 }),
            makeTask({ id: 't2', title: 'Legacy stored task', status: 'done', timeSpent: 5400, est: 60 }),
            makeTask({ id: 't3', title: 'Zero time task', status: 'todo' }),
        ];
        const history: StopwatchHistoryEntry[] = [
            makeWorkEntry(new Date(2026, 10, 27, 14, 0), 40, 't1'),
            makeWorkEntry(new Date(2026, 10, 27, 15, 0), 20, 't1'),
            makeBreakEntry(new Date(2026, 10, 27, 15, 30), 15),
        ];

        const stats = computeStats({ tasks, events: [], history, now });

        expect(stats.totalWorkSeconds).toBe(3600);
        expect(stats.totalTaskTimeSpentSeconds).toBe(9000);
        expect(stats.totalTrackedSeconds).toBe(9000);
        expect(stats.totalBreakSeconds).toBe(900);
        expect(stats.focusRatioPct).toBe(91);
        expect(stats.sessionCount).toBe(2);
        expect(stats.avgSessionMinutes).toBe(30);
        expect(stats.topTasksByTime).toHaveLength(2);
        expect(stats.topTasksByTime[0]).toMatchObject({
            id: 't2',
            trackedSeconds: 5400,
            estMinutes: 60,
        });
        expect(stats.topTasksByTime[1]).toMatchObject({
            id: 't1',
            trackedSeconds: 3600,
            estMinutes: 90,
        });
    });

    it('buckets 14-day activity and peak hours using local calendar boundaries', () => {
        const now = new Date(2026, 10, 27, 23, 50, 0);
        const lateTonight = new Date(2026, 10, 27, 23, 15, 0);
        const fourteenDaysAgo = new Date(2026, 10, 13, 14, 0, 0);
        const tasks: AppTask[] = [
            makeTask({ id: 't1', title: 'Late completion', status: 'done', completedAt: lateTonight.toISOString() }),
            makeTask({ id: 't2', title: 'Outside 14d window', status: 'done', completedAt: fourteenDaysAgo.toISOString() }),
        ];
        const history: StopwatchHistoryEntry[] = [
            makeWorkEntry(new Date(2026, 10, 27, 23, 5), 35, 't1'),
            makeWorkEntry(new Date(2026, 10, 14, 10, 0), 25),
        ];

        const stats = computeStats({ tasks, events: [], history, now });

        expect(stats.dailyActivity[0]).toMatchObject({
            date: '2026-11-14',
            trackedMinutes: 25,
            completedCount: 0,
        });
        expect(stats.dailyActivity[13]).toMatchObject({
            date: '2026-11-27',
            trackedMinutes: 35,
            completedCount: 1,
        });
        expect(stats.peakHour).toBe(23);
        expect(stats.peakHourLabel).toBe('23:00 – 00:00');
    });

    it('aggregates tags, priorities, statuses, and scheduled task blocks', () => {
        const now = new Date(2026, 10, 27, 12, 0, 0);
        const tasks: AppTask[] = [
            makeTask({
                id: 't1',
                title: 'Urgent API',
                status: 'done',
                priority: 4,
                timeSpent: 1800,
                tags: [{ id: '1', name: 'backend', color: '#8ab4f8' }, { id: '2', name: 'deep' }],
            }),
            makeTask({
                id: 't2',
                title: 'Next item',
                status: 'next-up',
                priority: 3,
                tags: [{ id: '1', name: 'backend' }],
            }),
            makeTask({
                id: 't3',
                title: 'Open item',
                status: 'todo',
                priority: 2,
            }),
            makeTask({
                id: 't4',
                title: 'Cancelled item',
                status: 'cancelled',
                priority: 1,
            }),
            makeTask({
                id: 't5',
                title: 'Unprioritized',
                status: 'done',
                priority: null,
            }),
        ];
        const events: AppEvent[] = [
            makeEvent({
                id: 'e1',
                title: 'API Block',
                taskId: 't1',
                tags: [{ id: '1', name: 'backend' }],
            }),
            makeEvent({
                id: 'e2',
                title: 'Standup',
                tags: [{ id: '3', name: 'meeting' }],
            }),
        ];

        const stats = computeStats({
            tasks,
            events,
            history: [],
            tags: ['backend', 'deep', 'meeting', 'unused'],
            now,
        });

        expect(stats.totalTasks).toBe(5);
        expect(stats.doneTasks).toBe(2);
        expect(stats.nextUpTasks).toBe(1);
        expect(stats.openTasks).toBe(1);
        expect(stats.cancelledTasks).toBe(1);
        expect(stats.completionRatePct).toBe(40);
        expect(stats.totalEvents).toBe(2);
        expect(stats.scheduledTaskBlocks).toBe(1);
        expect(stats.totalKnownTags).toBe(4);

        expect(stats.topTags[0]).toEqual({
            name: 'backend',
            color: '#8ab4f8',
            taskCount: 2,
            eventCount: 1,
            totalCount: 3,
            doneCount: 1,
            trackedSeconds: 1800,
        });

        const urgent = stats.priorityBreakdown.find((row) => row.priority === 4);
        const none = stats.priorityBreakdown.find((row) => row.priority === 0);
        expect(urgent).toMatchObject({ label: 'Urgent', total: 1, done: 1, completionPct: 100 });
        expect(none).toMatchObject({ label: 'None', total: 1, done: 1, completionPct: 100 });
    });
});
