import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import type { AppEvent, AppTask, StopwatchHistoryEntry } from '../../lib/domain';
import StatsScreen from './StatsScreen.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

function buildFixtureData(): {
    tasks: AppTask[];
    events: AppEvent[];
    history: StopwatchHistoryEntry[];
    tags: string[];
} {
    const now = new Date();
    const todayPeak = new Date(now.getFullYear(), now.getMonth(), now.getDate(), 14, 0, 0);
    const yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1, 14, 0, 0);

    const tasks: AppTask[] = [
        {
            id: 'task-1',
            title: 'Write stats engine',
            status: 'done',
            priority: 4,
            est: 60,
            timeSpent: 5400,
            completedAt: todayPeak.toISOString(),
            tags: [{ id: 'tag-1', name: 'feature', color: '#8ab4f8' }],
        },
        {
            id: 'task-2',
            title: 'Review design tokens',
            status: 'done',
            priority: 3,
            est: 30,
            timeSpent: 1800,
            completedAt: yesterday.toISOString(),
            tags: [{ id: 'tag-1', name: 'feature' }, { id: 'tag-2', name: 'ui' }],
        },
        {
            id: 'task-3',
            title: 'Polish mobile layout',
            status: 'next-up',
            priority: 2,
        },
    ];

    const events: AppEvent[] = [
        {
            id: 'ev-1',
            title: 'Stats focus block',
            taskId: 'task-1',
            startTime: `${now.getFullYear()}-01-01T14:00:00`,
            endTime: `${now.getFullYear()}-01-01T15:00:00`,
            tags: [{ id: 'tag-1', name: 'feature' }],
        },
    ];

    const history: StopwatchHistoryEntry[] = [
        {
            kind: 'work',
            taskId: 'task-1',
            label: 'Write stats engine',
            labelSource: 'task',
            startedAt: todayPeak.getTime(),
            endedAt: todayPeak.getTime() + 90 * 60_000,
            durationMs: 90 * 60_000,
        },
        {
            kind: 'break',
            taskId: null,
            label: 'Break',
            labelSource: 'unset',
            startedAt: todayPeak.getTime() + 95 * 60_000,
            endedAt: todayPeak.getTime() + 105 * 60_000,
            durationMs: 10 * 60_000,
        },
    ];

    return { tasks, events, history, tags: ['feature', 'ui'] };
}

describe('StatsScreen', () => {
    beforeEach(() => {
        tauri.reset();
        vi.clearAllMocks();
    });

    it('renders computed KPIs, top tasks, tags, peak hours, and breakdowns in the bento grid', async () => {
        const fixture = buildFixtureData();
        tauri.stubCommandValue('query_tasks', fixture.tasks);
        tauri.stubCommandValue('query_events', fixture.events);
        tauri.stubCommandValue('get_stopwatch_history', fixture.history);
        tauri.stubCommandValue('list_tags', fixture.tags);

        const { container } = render(StatsScreen);

        await waitFor(() => {
            expect(container.querySelector('[data-testid="kpi-focus-time"]')?.textContent).toContain('2h');
        });

        expect(container.querySelector('[data-testid="kpi-tasks-completed"]')?.textContent).toContain('2');
        expect(container.querySelector('[data-testid="kpi-tasks-completed"]')?.textContent).toContain('67%');
        expect(container.querySelector('[data-testid="kpi-streak"]')?.textContent).toContain('2d');
        expect(container.querySelector('[data-testid="kpi-focus-ratio"]')?.textContent).toContain('92%');

        expect(container.querySelector('[data-testid="card-peak-hours"]')?.textContent).toContain('14:00 – 15:00');
        expect(container.querySelector('[data-testid="card-top-tasks"]')?.textContent).toContain('Write stats engine');
        expect(container.querySelector('[data-testid="card-top-tasks"]')?.textContent).toContain('est 1h');
        expect(container.querySelector('[data-testid="card-tags"]')?.textContent).toContain('#feature');
        expect(container.querySelector('[data-testid="card-status"]')?.textContent).toContain('Next Up');
        expect(container.querySelector('[data-testid="card-priority"]')?.textContent).toContain('Urgent');

        const dayButtons = container.querySelectorAll('.activity-day');
        expect(dayButtons).toHaveLength(14);
        await fireEvent.focus(dayButtons[13]!);
        expect(container.querySelector('.activity-readout')?.textContent).toContain('1h 30m tracked');
    });

    it('renders friendly zero states when there is no history, tasks, or events', async () => {
        tauri.stubCommandValue('query_tasks', []);
        tauri.stubCommandValue('query_events', []);
        tauri.stubCommandValue('get_stopwatch_history', []);
        tauri.stubCommandValue('list_tags', []);

        const { container } = render(StatsScreen);

        await waitFor(() => {
            expect(container.querySelector('[data-testid="kpi-focus-time"]')?.textContent).toContain('0m');
        });

        expect(container.textContent).not.toContain('NaN');
        expect(screen.getByText('No task focus time recorded yet.')).toBeTruthy();
        expect(screen.getByText('No tagged tasks or events yet.')).toBeTruthy();
        expect(screen.getByText('No tasks yet.')).toBeTruthy();
        expect(screen.getByText('No activity yet')).toBeTruthy();
    });

    it('displays a non-fatal alert banner if a stats query fails', async () => {
        tauri.stubCommand('query_tasks', () => Promise.reject({ code: 'remote', message: 'CalDAV offline' }));
        tauri.stubCommandValue('query_events', []);
        tauri.stubCommandValue('get_stopwatch_history', []);
        tauri.stubCommandValue('list_tags', []);

        render(StatsScreen);

        const alert = await screen.findByRole('alert');
        expect(alert.textContent).toContain('CalDAV offline');
    });
});
