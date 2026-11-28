import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { toast } from 'svelte-sonner';
import { mockTauriInvoke } from '../../test/tauriMock';
import { makeTaskDateInstance } from '../../test/factories';
import { store } from '../../lib/store.svelte';
import type { AppCalendar } from '../../lib/domain';
import { addDays, startOfWeek, ymd } from '../../lib/time';
import PlanScreen from './PlanScreen.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

vi.mock('svelte-sonner', () => ({
    toast: {
        error: vi.fn(),
        success: vi.fn(),
        info: vi.fn(),
        custom: vi.fn(),
    },
    Toaster: vi.fn(),
}));

const tauri = mockTauriInvoke();

const task = {
    id: 'task-1',
    title: 'Buy milk',
    status: 'todo' as const,
};

describe('PlanScreen task list', () => {
    beforeEach(async () => {
        await page.viewport(1200, 800);
        tauri.reset();
        vi.clearAllMocks();
        store.loaded = true;
        store.error = null;
        tauri.stubCommandValue('query_tasks', [task]);
        tauri.stubCommandValue('get_past_due_task_ids', []);
        tauri.stubCommandValue('get_task_schema', []);
        tauri.stubCommandValue('get_active_calendars', []);
        tauri.stubCommandValue('query_event_instances', []);
        tauri.stubCommandValue('get_ui_state', undefined);
    });

    afterEach(() => {
        store.loaded = false;
        store.settings = null;
    });

    it('completes the task when its circle is clicked', async () => {
        render(PlanScreen);

        await screen.findByText('Buy milk');
        const row = screen.getByText('Buy milk').closest('.task-row');
        const circle = row?.querySelector('.task-circle');
        expect(circle).not.toBeNull();

        await fireEvent.click(circle!);

        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'update_task',
                {
                    id: 'task-1',
                    patches: [{ field: 'status', value: 'done' }],
                },
                undefined,
            );
        });
        const calendarCalls = tauri.invoke.mock.calls.filter(([name]) => name === 'get_active_calendars');
        expect(calendarCalls).toHaveLength(1);
    });

    it('toasts a failed update instead of replacing the screen with a fatal error', async () => {
        tauri.stubCommand('update_task', () => Promise.reject({ code: 'remote', message: 'server down' }));
        render(PlanScreen);

        await screen.findByText('Buy milk');
        const row = screen.getByText('Buy milk').closest('.task-row');
        const circle = row?.querySelector('.task-circle');
        expect(circle).not.toBeNull();

        await fireEvent.click(circle!);

        await waitFor(() => {
            expect(toast.custom).toHaveBeenCalledTimes(1);
        });
        expect(toast.custom).toHaveBeenCalledWith(
            expect.anything(),
            expect.objectContaining({
                componentProps: { message: 'Failed to update task: server down' },
                duration: Infinity,
            }),
        );
        expect(store.error).toBeNull();
        expect(screen.getByText('Buy milk')).toBeTruthy();
        expect(screen.queryByText(/Error loading data from backend/)).toBeNull();
    });

    it('leaves remoteCollectionId unset when adding a task so Rust honors default_task_list', async () => {
        tauri.stubCommandValue('get_active_calendars', [
            { id: 'tasks-first', summary: 'Personal Tasks', writable: true, component: 'VTODO' },
            { id: 'tasks-default', summary: 'Work Tasks', writable: true, component: 'VTODO' },
        ]);
        store.settings = { default_task_list: 'tasks-default' };

        render(PlanScreen);
        await screen.findByText('Buy milk');

        await fireEvent.click(screen.getByTitle('Add Task'));

        await waitFor(() => {
            const createCall = tauri.invoke.mock.calls.find(([name]) => name === 'create_task');
            expect(createCall).toBeDefined();
            const created = (
                createCall?.[1] as { task: { title: string; remoteCollectionId?: string } } | undefined
            )?.task;
            expect(created?.title).toBe('New Task');
            expect(created?.remoteCollectionId).toBeUndefined();
        });
    });
});

async function dragElementTo(source: Element, targetX: number, targetY: number) {
    const rect = source.getBoundingClientRect();
    const startX = rect.left + rect.width / 2;
    const startY = rect.top + rect.height / 2;
    await fireEvent.pointerDown(source, { button: 0, clientX: startX, clientY: startY });
    window.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientX: startX + 10, clientY: startY + 10 }));
    window.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientX: targetX, clientY: targetY }));
    window.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, clientX: targetX, clientY: targetY }));
}

function cellCenter(container: HTMLElement, date: string): { x: number; y: number } {
    const cell = container.querySelector<HTMLElement>(`[data-drop-kind="grid-day"][data-drop-date="${date}"]`);
    if (!cell) throw new Error(`Missing day cell for ${date}`);
    const rect = cell.getBoundingClientRect();
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}

describe('PlanScreen date-grid task drag and drop', () => {
    const calendars: AppCalendar[] = [
        { id: 'events', summary: 'Events', writable: true, component: 'VEVENT' },
        { id: 'tasks', summary: 'Tasks', writable: true, component: 'VTODO' },
        { id: 'ro-tasks', summary: 'Read-only Tasks', writable: false, component: 'VTODO' },
    ];

    let weekStart: Date;
    let day0: string;
    let day1: string;
    let day2: string;
    let host: HTMLDivElement;

    beforeEach(async () => {
        await page.viewport(1200, 800);
        tauri.reset();
        vi.clearAllMocks();
        store.loaded = true;
        store.error = null;
        store.settings = null;

        weekStart = startOfWeek(new Date());
        day0 = ymd(weekStart);
        day1 = ymd(addDays(weekStart, 1));
        day2 = ymd(addDays(weekStart, 2));

        const timedStart = new Date(weekStart);
        timedStart.setHours(9, 30, 0, 0);

        tauri.stubCommandValue('get_active_calendars', calendars);
        tauri.stubCommandValue('get_past_due_task_ids', []);
        tauri.stubCommandValue('get_task_schema', []);
        tauri.stubCommandValue('query_event_instances', []);
        tauri.stubCommandValue('query_events', []);
        tauri.stubCommandValue('list_tags', []);
        tauri.stubCommandValue('get_ui_state', undefined);
        tauri.stubCommandValue('query_tasks', [
            { id: 't-start', title: 'All-day start task', status: 'todo', remoteCollectionId: 'tasks', start: day0 },
            { id: 't-timed', title: 'Timed start task', status: 'todo', remoteCollectionId: 'tasks', start: `${day0}T09:30:00` },
            { id: 't-due', title: 'Due task', status: 'todo', remoteCollectionId: 'tasks', due: day1 },
            { id: 't-ro', title: 'Readonly marker', status: 'todo', remoteCollectionId: 'ro-tasks', start: day0 },
        ]);
        tauri.stubCommandValue('query_task_dates', [
            makeTaskDateInstance({
                taskId: 't-start',
                kind: 'start',
                title: 'All-day start task',
                calendarId: 'tasks',
                timing: { kind: 'allDay', date: day0 },
            }),
            makeTaskDateInstance({
                taskId: 't-timed',
                kind: 'start',
                title: 'Timed start task',
                calendarId: 'tasks',
                timing: { kind: 'timed', start: timedStart.toISOString(), timezone: null },
            }),
            makeTaskDateInstance({
                taskId: 't-due',
                kind: 'due',
                title: 'Due task',
                calendarId: 'tasks',
                timing: { kind: 'allDay', date: day1 },
            }),
            makeTaskDateInstance({
                taskId: 't-ro',
                kind: 'start',
                title: 'Readonly marker',
                calendarId: 'ro-tasks',
                timing: { kind: 'allDay', date: day0 },
            }),
        ]);

        host = document.createElement('div');
        host.style.width = '1200px';
        host.style.height = '800px';
        document.body.appendChild(host);
    });

    afterEach(() => {
        host?.remove();
        store.loaded = false;
    });

    it('moves start and due task capsules to a new day while preserving timed HH:MM:00', async () => {
        const { container } = render(PlanScreen, { target: host });
        await waitFor(() => {
            expect(container.querySelector('.day-cell-task')).not.toBeNull();
        });

        const dropDay2 = cellCenter(container, day2);

        const startCapsule = container.querySelector('[title="Start: All-day start task"]')!;
        await dragElementTo(startCapsule, dropDay2.x, dropDay2.y);

        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'update_task',
                { id: 't-start', patches: [{ field: 'start', value: day2 }] },
                undefined,
            );
        });

        const timedCapsule = container.querySelector('[title="Start: Timed start task"]')!;
        await dragElementTo(timedCapsule, dropDay2.x, dropDay2.y);

        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'update_task',
                { id: 't-timed', patches: [{ field: 'start', value: `${day2}T09:30:00` }] },
                undefined,
            );
        });

        const dueCapsule = container.querySelector('[title="Due: Due task"]')!;
        await dragElementTo(dueCapsule, dropDay2.x, dropDay2.y);

        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'update_task',
                { id: 't-due', patches: [{ field: 'due', value: day2 }] },
                undefined,
            );
        });
    });

    it('treats dropping on the same day or dragging a read-only marker as a no-op', async () => {
        const { container } = render(PlanScreen, { target: host });
        await waitFor(() => {
            expect(container.querySelector('[title="Start: All-day start task"]')).not.toBeNull();
        });

        const dropDay0 = cellCenter(container, day0);
        const dropDay2 = cellCenter(container, day2);

        await dragElementTo(container.querySelector('[title="Start: All-day start task"]')!, dropDay0.x, dropDay0.y);
        await dragElementTo(container.querySelector('[title="Start: Readonly marker"]')!, dropDay2.x, dropDay2.y);

        const updateCalls = tauri.invoke.mock.calls.filter(([name]) => name === 'update_task');
        expect(updateCalls).toHaveLength(0);
    });

    it('schedules a linked event when dropping a date-grid task capsule onto the timeline without bypassing default_event_list', async () => {
        tauri.stubCommandValue('get_active_calendars', [
            { id: 'events-first', summary: 'Personal', writable: true, component: 'VEVENT' },
            { id: 'events-default', summary: 'Work', writable: true, component: 'VEVENT' },
            { id: 'tasks', summary: 'Tasks', writable: true, component: 'VTODO' },
        ]);
        store.settings = { default_event_list: 'events-default' };

        const { container } = render(PlanScreen, { target: host });
        const todayStr = ymd(new Date());
        const timelineScroller = container.querySelector<HTMLElement>('.day-scroll')!;
        await waitFor(() => {
            expect(container.querySelector('[title="Due: Due task"]')).not.toBeNull();
            expect(
                container.querySelector(`[data-drop-kind="day-time"][data-drop-date="${todayStr}"]`),
            ).not.toBeNull();
        });

        const timelineCol = container.querySelector<HTMLElement>(
            `[data-drop-kind="day-time"][data-drop-date="${todayStr}"]`,
        )!;
        const scrollerRect = timelineScroller.getBoundingClientRect();
        const colRect = timelineCol.getBoundingClientRect();
        const dueCapsule = container.querySelector('[title="Due: Due task"]')!;

        await dragElementTo(
            dueCapsule,
            colRect.left + colRect.width / 2,
            Math.max(colRect.top + 40, scrollerRect.top + 80),
        );

        await waitFor(() => {
            const createCall = tauri.invoke.mock.calls.find(([name]) => name === 'create_event');
            expect(createCall?.[1]).toMatchObject({
                event: {
                    title: 'Due task',
                    taskId: 't-due',
                    isAllDay: false,
                },
            });
            const created = (
                createCall?.[1] as { event: { remoteCollectionId?: string } } | undefined
            )?.event;
            expect(created?.remoteCollectionId).toBeUndefined();
        });
    });

    it('opens inspector on click and toggles completion on circle click without dragging', async () => {
        const { container } = render(PlanScreen, { target: host });
        await waitFor(() => {
            expect(container.querySelector('[title="Start: All-day start task"]')).not.toBeNull();
        });

        const startCapsule = container.querySelector<HTMLElement>('[title="Start: All-day start task"]')!;
        const circle = startCapsule.querySelector<HTMLElement>('.task-circle')!;

        await fireEvent.pointerDown(circle, { button: 0 });
        await fireEvent.pointerUp(circle, { button: 0 });
        await fireEvent.click(circle);

        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith('toggle_task', { id: 't-start' }, undefined);
        });
        expect(container.querySelector('.inspector-pane')?.getAttribute('aria-hidden')).toBe('true');

        await fireEvent.pointerDown(startCapsule, { button: 0 });
        await fireEvent.pointerUp(startCapsule, { button: 0 });
        await fireEvent.click(startCapsule);

        await waitFor(() => {
            expect(screen.getByRole('textbox', { name: 'Title' })).toBeTruthy();
        });
    });
});
