import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import { makeTaskDateInstance } from '../../test/factories';
import { store } from '../../lib/store.svelte';
import { persistedKeys } from '../../lib/persisted.svelte';
import type { AppTask, StopwatchHistoryEntry } from '../../lib/domain';
import WrapScreen from './WrapScreen.svelte';
import StageIndicator from '../../components/shell/StageIndicator.svelte';
import MoreScreensDropdown from '../../components/shell/MoreScreensDropdown.svelte';

const { gotoMock, pageState } = vi.hoisted(() => ({
    gotoMock: vi.fn(),
    pageState: { url: { pathname: '/' } },
}));

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

vi.mock('../../lib/useNow.svelte', () => ({
    useNow: () => ({
        get value() { return new Date(); },
        get ms() { return Date.now(); },
    }),
}));

vi.mock('$app/navigation', () => ({
    goto: gotoMock,
}));

vi.mock('$app/paths', () => ({
    resolve: (path: string) => path,
}));

vi.mock('$app/state', () => ({
    page: pageState,
}));

const tauri = mockTauriInvoke();

describe('WrapScreen & navigation visibility', () => {
    beforeEach(() => {
        vi.useFakeTimers({ toFake: ['Date'] });
        tauri.reset();
        vi.clearAllMocks();
        pageState.url.pathname = '/';
        store.settings = { earliest_wake_up_time: 7 * 60 };
        tauri.stubCommandValue('query_tasks', []);
        tauri.stubCommandValue('query_task_dates', []);
        tauri.stubCommandValue('get_stopwatch_history', []);
        tauri.stubCommandValue('get_ui_state', undefined);
    });

    afterEach(() => {
        vi.useRealTimers();
        store.settings = null;
    });

    it('hides wrap from navigation and shows the closed state outside 10 PM - 3 AM', async () => {
        vi.setSystemTime(new Date(2026, 10, 26, 14, 0, 0));

        const stageRender = render(StageIndicator);
        expect(screen.queryByText('wrap')).toBeNull();
        stageRender.unmount();

        const menuRender = render(MoreScreensDropdown);
        await fireEvent.click(screen.getByTitle('More screens'));
        expect(screen.queryByText('wrap')).toBeNull();
        expect(screen.getByText('stats')).toBeTruthy();
        menuRender.unmount();

        render(WrapScreen);
        expect(screen.getByText('Wrap opens at 10:00 PM')).toBeTruthy();
        await fireEvent.click(screen.getByRole('button', { name: 'Back to Plan' }));
        expect(gotoMock).toHaveBeenCalledWith('/');
    });

    it('surfaces wrap in shell navigation and shows today at 11 PM with timeline blocks', async () => {
        vi.setSystemTime(new Date(2026, 10, 26, 23, 0, 0));

        const tasks: AppTask[] = [
            {
                id: 'done-1',
                title: 'Finish quarterly report',
                status: 'done',
                completedAt: new Date(2026, 10, 26, 17, 15, 0).toISOString(),
                est: 30,
            },
            {
                id: 'done-2',
                title: 'Inbox zero',
                status: 'done',
                completedAt: new Date(2026, 10, 26, 19, 30, 0).toISOString(),
                est: 25,
            },
            {
                id: 'next-1',
                title: 'Prepare demo slides',
                status: 'next-up',
                start: '2026-11-26',
            },
            {
                id: 'todo-1',
                title: 'Refactor auth helper',
                status: 'todo',
            },
        ];
        const history: StopwatchHistoryEntry[] = [
            {
                kind: 'work',
                taskId: 'done-1',
                label: 'Finish quarterly report',
                labelSource: 'task',
                startedAt: new Date(2026, 10, 26, 16, 0, 0).getTime(),
                endedAt: new Date(2026, 10, 26, 17, 0, 0).getTime(),
                durationMs: 60 * 60_000,
            },
        ];
        tauri.stubCommandValue('query_tasks', tasks);
        tauri.stubCommandValue('get_stopwatch_history', history);

        pageState.url.pathname = '/wrap';
        const stageRender = render(StageIndicator);
        expect(screen.getByRole('button', { name: 'wrap' })).toBeTruthy();
        stageRender.unmount();

        const menuRender = render(MoreScreensDropdown);
        await fireEvent.click(screen.getByTitle('More screens'));
        expect(screen.getByText('wrap')).toBeTruthy();
        menuRender.unmount();

        const { container } = render(WrapScreen);

        await waitFor(() => {
            expect(screen.getByText('Thu, Nov 26')).toBeTruthy();
            expect(container.querySelector('[data-wrap-item="work"]')).not.toBeNull();
            expect(container.querySelector('[data-wrap-item="completion-pin"]')).not.toBeNull();
            expect(container.querySelector('[data-wrap-item="completed"]')).not.toBeNull();
        });
        expect(screen.queryByText('(Yesterday)')).toBeNull();
        expect(screen.getByText('✓ 2 tasks done')).toBeTruthy();
        expect(screen.getByText('1h focused')).toBeTruthy();
    });

    it('shows yesterday at 12:30 AM and supports next-up actions plus tomorrow planning', async () => {
        vi.setSystemTime(new Date(2026, 10, 27, 0, 30, 0));

        const tasks: AppTask[] = [
            {
                id: 'done-yest',
                title: 'Shipped feature',
                status: 'done',
                completedAt: new Date(2026, 10, 26, 20, 0, 0).toISOString(),
            },
            {
                id: 'next-yest',
                title: 'Carry-over task',
                status: 'next-up',
                start: '2026-11-26',
            },
            {
                id: 'next-backlog',
                title: 'Maybe later task',
                status: 'next-up',
                start: '2026-11-26',
            },
            {
                id: 'todo-pick',
                title: 'Pick me for tomorrow',
                status: 'todo',
            },
        ];
        tauri.stubCommandValue('query_tasks', tasks);

        render(WrapScreen);

        await waitFor(() => {
            expect(screen.getByText('Thu, Nov 26')).toBeTruthy();
            expect(screen.getByText('(Yesterday)')).toBeTruthy();
            expect(screen.getByText('Carry-over task')).toBeTruthy();
        });

        await fireEvent.click(screen.getAllByRole('button', { name: 'Tomorrow' })[0]!);
        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'update_task',
                {
                    id: 'next-yest',
                    patches: [{ field: 'start', value: '2026-11-27' }],
                },
                undefined,
            );
        });

        await fireEvent.click(screen.getAllByRole('button', { name: 'Backlog' })[1]!);
        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'update_task',
                {
                    id: 'next-backlog',
                    patches: [{ field: 'status', value: 'todo' }],
                },
                undefined,
            );
        });

        await fireEvent.click(screen.getByText('Pick me for tomorrow'));
        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'update_task',
                {
                    id: 'todo-pick',
                    patches: [
                        { field: 'status', value: 'next-up' },
                        { field: 'start', value: '2026-11-27' },
                    ],
                },
                undefined,
            );
        });

        await fireEvent.click(screen.getAllByRole('button', { name: 'Done' })[0]!);
        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith('toggle_task', { id: 'next-yest' }, undefined);
        });

        const newTaskInput = screen.getByLabelText('New task for tomorrow');
        await fireEvent.input(newTaskInput, { target: { value: 'Fresh morning priority' } });
        await fireEvent.click(screen.getByRole('button', { name: 'Add' }));
        await waitFor(() => {
            const createCall = tauri.invoke.mock.calls.find(([name]) => name === 'create_task');
            expect(createCall?.[1]).toMatchObject({
                task: {
                    title: 'Fresh morning priority',
                    status: 'next-up',
                    start: '2026-11-27',
                },
            });
            const created = (createCall?.[1] as { task: AppTask } | undefined)?.task;
            expect(created?.remoteCollectionId).toBeUndefined();
        });
    });

    it('persists mood, energy, and reflection text for the wrap date', async () => {
        vi.setSystemTime(new Date(2026, 10, 26, 22, 15, 0));

        const { unmount } = render(WrapScreen);
        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'get_ui_state',
                { key: persistedKeys.wrapReflections },
                undefined,
            );
        });
        await new Promise((resolve) => setTimeout(resolve, 0));

        await fireEvent.click(screen.getByRole('button', { name: /4\s+Good/ }));
        await fireEvent.click(screen.getByRole('button', { name: /3\s+Steady/ }));
        await fireEvent.input(screen.getByLabelText('What went well today?'), {
            target: { value: 'Finished the deep work block.' },
        });
        await fireEvent.input(screen.getByLabelText('How could productivity improve tomorrow?'), {
            target: { value: 'Start earlier without checking chat.' },
        });

        unmount();

        expect(tauri.invoke).toHaveBeenCalledWith(
            'set_ui_state',
            {
                key: persistedKeys.wrapReflections,
                value: {
                    '2026-11-26': {
                        mood: 4,
                        energy: 3,
                        wentWell: 'Finished the deep work block.',
                        improve: 'Start earlier without checking chat.',
                    },
                },
            },
            undefined,
        );
    });

    it('slates tomorrow tasks from projected query_task_dates and clears start when unplanning', async () => {
        vi.setSystemTime(new Date(2026, 10, 26, 22, 30, 0));

        const tomorrowLocalStart = new Date(2026, 10, 27, 9, 0, 0).toISOString();
        const tasks: AppTask[] = [
            {
                id: 'planned-tom',
                title: 'Morning review',
                status: 'next-up',
                start: tomorrowLocalStart,
            },
            {
                id: 'backlog-due',
                title: 'Later deadline',
                status: 'todo',
                due: '2026-11-29T23:30:00Z',
                timezone: 'Asia/Kolkata',
            },
        ];
        tauri.stubCommandValue('query_tasks', tasks);
        tauri.stubCommandValue('query_task_dates', [
            makeTaskDateInstance({
                taskId: 'planned-tom',
                kind: 'start',
                title: 'Morning review',
                status: 'next-up',
                timing: { kind: 'timed', start: tomorrowLocalStart, timezone: null },
            }),
        ]);

        render(WrapScreen);

        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'query_task_dates',
                { startDate: '2026-11-27', endDate: '2026-11-27' },
                undefined,
            );
            expect(screen.getByText('Morning review')).toBeTruthy();
        });
        expect(screen.getByText('due 2026-11-30')).toBeTruthy();

        await fireEvent.click(screen.getByRole('button', { name: 'Unplan' }));
        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'update_task',
                {
                    id: 'planned-tom',
                    patches: [
                        { field: 'status', value: 'todo' },
                        { field: 'start', value: null },
                    ],
                },
                undefined,
            );
        });
    });
});
