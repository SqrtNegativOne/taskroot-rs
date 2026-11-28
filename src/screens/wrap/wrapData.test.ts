import { describe, expect, it } from 'vitest';
import type { AppTask, StopwatchHistoryEntry } from '../../lib/domain';
import { buildWrapDaySummary, formatWrapDuration } from './wrapData';

function makeTask(overrides: Partial<AppTask> & { id: string; title: string }): AppTask {
    return {
        status: 'todo',
        ...overrides,
    };
}

function makeWorkEntry(
    start: Date,
    end: Date,
    overrides: Partial<StopwatchHistoryEntry> = {},
): StopwatchHistoryEntry {
    return {
        kind: 'work',
        taskId: null,
        label: 'Focus',
        labelSource: 'custom',
        startedAt: start.getTime(),
        endedAt: end.getTime(),
        durationMs: end.getTime() - start.getTime(),
        ...overrides,
    };
}

describe('buildWrapDaySummary', () => {
    const wrapDay = new Date(2026, 10, 26);

    it('includes only tasks completed on wrapDate in local time', () => {
        const doneToday = makeTask({
            id: 't1',
            title: 'Write wrap tests',
            status: 'done',
            completedAt: new Date(2026, 10, 26, 16, 30, 0).toISOString(),
            est: 40,
        });
        const doneYesterday = makeTask({
            id: 't2',
            title: 'Yesterday task',
            status: 'done',
            completedAt: new Date(2026, 10, 25, 23, 30, 0).toISOString(),
        });
        const doneTomorrow = makeTask({
            id: 't3',
            title: 'Tomorrow task',
            status: 'done',
            completedAt: new Date(2026, 10, 27, 0, 15, 0).toISOString(),
        });
        const stillOpen = makeTask({
            id: 't4',
            title: 'Still open',
            status: 'next-up',
        });

        const summary = buildWrapDaySummary([doneYesterday, doneToday, doneTomorrow, stillOpen], [], wrapDay);

        expect(summary.completedCount).toBe(1);
        expect(summary.completedTasks.map((t) => t.id)).toEqual(['t1']);
        expect(summary.items).toHaveLength(1);
        expect(summary.items[0]).toMatchObject({
            kind: 'completed',
            taskId: 't1',
            title: 'Write wrap tests',
            startMins: 16 * 60 + 30 - 40,
            endMins: 16 * 60 + 30,
            isDone: true,
        });
    });

    it('filters to work entries only, clips cross-midnight sessions to wrapDate, and sums clipped duration', () => {
        const overnightWork = makeWorkEntry(
            new Date(2026, 10, 25, 23, 0, 0),
            new Date(2026, 10, 26, 1, 30, 0),
            { label: 'Late debugging' },
        );
        const breakEntry: StopwatchHistoryEntry = {
            kind: 'break',
            taskId: null,
            label: 'Tea',
            labelSource: 'unset',
            startedAt: new Date(2026, 10, 26, 10, 0, 0).getTime(),
            endedAt: new Date(2026, 10, 26, 10, 15, 0).getTime(),
            durationMs: 15 * 60_000,
        };
        const dayWork = makeWorkEntry(
            new Date(2026, 10, 26, 22, 30, 0),
            new Date(2026, 10, 27, 0, 30, 0),
            { label: 'Wrap up' },
        );

        const summary = buildWrapDaySummary([], [overnightWork, breakEntry, dayWork], wrapDay, 8 * 60);

        expect(summary.workSessionCount).toBe(2);
        // 1h30m after midnight + 1h30m before next midnight = 3h
        expect(summary.totalTrackedMs).toBe(3 * 60 * 60_000);
        expect(summary.items.map((item) => [item.title, item.startMins, item.endMins])).toEqual([
            ['Late debugging', 0, 90],
            ['Wrap up', 22 * 60 + 30, 24 * 60],
        ]);
        // Early item at 00:00 pulls startHour down below the 8 AM wake-up time.
        expect(summary.startHour).toBe(0);
    });

    it('renders both tracked work blocks and a completion pin when a completed task has focus sessions', () => {
        const task = makeTask({
            id: 't-work',
            title: 'Ship Wrap screen',
            status: 'done',
            completedAt: new Date(2026, 10, 26, 15, 0, 0).toISOString(),
            timeSpent: 3600,
        });
        const work = makeWorkEntry(
            new Date(2026, 10, 26, 14, 0, 0),
            new Date(2026, 10, 26, 15, 0, 0),
            { taskId: 't-work', label: 'Old label' },
        );

        const summary = buildWrapDaySummary([task], [work], wrapDay, 7 * 60);

        expect(summary.startHour).toBe(7);
        expect(summary.items).toHaveLength(2);
        expect(summary.items[0]).toMatchObject({
            kind: 'work',
            title: 'Ship Wrap screen',
            startMins: 14 * 60,
            endMins: 15 * 60,
            isDone: true,
        });
        expect(summary.items[1]).toMatchObject({
            kind: 'completion-pin',
            title: 'Ship Wrap screen',
            completedAtMins: 15 * 60,
            isDone: true,
        });
    });

    it('assigns non-overlapping lanes to concurrent items', () => {
        const w1 = makeWorkEntry(
            new Date(2026, 10, 26, 10, 0, 0),
            new Date(2026, 10, 26, 11, 0, 0),
            { label: 'Session A' },
        );
        const t2 = makeTask({
            id: 't2',
            title: 'Quick win',
            status: 'done',
            completedAt: new Date(2026, 10, 26, 10, 45, 0).toISOString(),
            est: 25,
        });

        const summary = buildWrapDaySummary([t2], [w1], wrapDay);

        expect(summary.items).toHaveLength(2);
        const lanes = summary.items.map((item) => item.lane);
        expect(new Set(lanes).size).toBe(2);
        expect(summary.items.every((item) => item.lanes === 2)).toBe(true);
    });

    it('uses wake-up hour when there are no items earlier than wake-up time', () => {
        const t = makeTask({
            id: 't1',
            title: 'Morning review',
            status: 'done',
            completedAt: new Date(2026, 10, 26, 9, 30, 0).toISOString(),
        });

        expect(buildWrapDaySummary([t], [], wrapDay, 7 * 60).startHour).toBe(7);
        expect(buildWrapDaySummary([], [], wrapDay, 6 * 60 + 30).startHour).toBe(6);
    });
});

describe('formatWrapDuration', () => {
    it('formats zero, sub-minute, minute, and hour+minute durations', () => {
        expect(formatWrapDuration(0)).toBe('0m');
        expect(formatWrapDuration(25_000)).toBe('< 1m');
        expect(formatWrapDuration(45 * 60_000)).toBe('45m');
        expect(formatWrapDuration(120 * 60_000)).toBe('2h');
        expect(formatWrapDuration(95 * 60_000)).toBe('1h 35m');
    });
});
