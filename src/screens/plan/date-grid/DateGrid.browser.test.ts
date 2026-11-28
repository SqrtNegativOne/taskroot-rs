import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../../test/tauriMock';
import { makeInstance, makeTaskDateInstance } from '../../../test/factories';
import { addDays, startOfWeek, ymd } from '../../../lib/time';
import { persistedKeys } from '../../../lib/persisted.svelte';
import { filterGroup } from '../../../lib/domain';
import type { AppCalendar } from '../../../lib/domain';
import DateGrid from './DateGrid.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();
const calendars: AppCalendar[] = [
    { id: 'events', summary: 'Event Calendar', writable: true, component: 'VEVENT' },
    { id: 'tasks', summary: 'Task List', writable: true, component: 'VTODO' },
];

async function openFilters() {
    await fireEvent.click(screen.getByTitle('Filter'));
    return {
        recurring: screen.getByRole('checkbox', { name: 'Hide daily or weekly recurring events' }),
        timed: screen.getByRole('checkbox', { name: 'Hide timed events' }),
        done: screen.getByRole('checkbox', { name: 'Hide completed tasks' }),
    };
}

describe('DateGrid visibility filters', () => {
    beforeEach(() => {
        tauri.reset();
        const start = startOfWeek(new Date());
        const startDate = ymd(start);
        const endDateExclusive = ymd(addDays(start, 1));
        const timedStart = new Date(start);
        timedStart.setHours(9);
        const timedEnd = new Date(start);
        timedEnd.setHours(10);
        const timing = { start: timedStart.toISOString(), end: timedEnd.toISOString() };
        tauri.stubCommandValue('get_active_calendars', calendars);
        tauri.stubCommandValue('get_ui_state', undefined);
        tauri.stubCommandValue('query_event_instances', [
            makeInstance({ id: 'all-day', title: 'One-off all day', timing: { kind: 'allDay', startDate, endDateExclusive } }),
            makeInstance({ id: 'timed', title: 'One-off timed', timing }),
            makeInstance({ id: 'repeat', title: 'Recurring timed', recurring: true, rrule: 'RRULE:FREQ=DAILY', timing }),
            makeInstance({ id: 'exception', title: 'Moved exception', recurring: true, rrule: 'RRULE:FREQ=WEEKLY', timing }),
            makeInstance({ id: 'span', title: 'Recurring span', recurring: true, rrule: 'RRULE:FREQ=WEEKLY', timing: { kind: 'allDay', startDate, endDateExclusive: ymd(addDays(start, 3)) } }),
            makeInstance({ id: 'yearly', title: 'Yearly event', recurring: true, rrule: 'RRULE:FREQ=YEARLY', timing }),
        ]);
    });

    it('shows all event kinds by default and offers only event calendars', async () => {
        render(DateGrid);
        await screen.findByText('One-off all day');
        expect(screen.getByText('One-off timed')).toBeTruthy();
        expect(screen.getByText('Recurring timed')).toBeTruthy();
        expect(screen.getByRole('button', { name: 'Recurring span' })).toBeTruthy();
        expect(screen.getByText('Yearly event')).toBeTruthy();
        const { recurring, timed, done } = await openFilters();
        expect(recurring).toHaveProperty('checked', false);
        expect(timed).toHaveProperty('checked', false);
        expect(done).toHaveProperty('checked', false);
        await fireEvent.click(screen.getByRole('button', { name: /add filter/i }));
        await fireEvent.click(screen.getByRole('button', { name: /select\.\.\./i }));
        expect(screen.getByRole('checkbox', { name: 'Event Calendar' })).toBeTruthy();
        expect(screen.queryByRole('checkbox', { name: 'Task List' })).toBeNull();
    });

    it('hides daily and weekly recurring occurrences, exceptions and spanning bars, but keeps less frequent series', async () => {
        render(DateGrid);
        await screen.findByText('Recurring timed');
        const { recurring } = await openFilters();
        await fireEvent.click(recurring);
        expect(screen.queryByText('Recurring timed')).toBeNull();
        expect(screen.queryByText('Moved exception')).toBeNull();
        expect(screen.queryByRole('button', { name: 'Recurring span' })).toBeNull();
        expect(screen.getByText('Yearly event')).toBeTruthy();
        expect(screen.getByText('One-off timed')).toBeTruthy();
        expect(screen.getByText('One-off all day')).toBeTruthy();
        await fireEvent.click(recurring);
        expect(screen.getByText('Moved exception')).toBeTruthy();
        expect(screen.getByRole('button', { name: 'Recurring span' })).toBeTruthy();
    });

    it('hides timed events independently and combines both choices', async () => {
        render(DateGrid);
        await screen.findByText('One-off timed');
        const { recurring, timed } = await openFilters();
        await fireEvent.click(timed);
        expect(screen.queryByText('One-off timed')).toBeNull();
        expect(screen.queryByText('Recurring timed')).toBeNull();
        expect(screen.getByRole('button', { name: 'Recurring span' })).toBeTruthy();
        await fireEvent.click(recurring);
        expect(screen.queryByRole('button', { name: 'Recurring span' })).toBeNull();
        expect(screen.getByText('One-off all day')).toBeTruthy();
        expect(screen.getByTitle('Filter').textContent?.trim()).toBe('2');
    });

    it('persists choices and restores them on remount without overwriting calendar filters', async () => {
        const stored = new Map<string, unknown>([
            [persistedKeys.dateGridFilters, filterGroup([{ column: 'remotecollectionid', operator: 'is', value: ['events'] }])],
        ]);
        tauri.stubCommand('get_ui_state', args => stored.get(String(args?.key)));
        tauri.stubCommand('set_ui_state', args => { stored.set(String(args?.key), args?.value); });
        const mounted = render(DateGrid);
        await screen.findByText('One-off timed');
        const { timed } = await openFilters();
        await fireEvent.click(timed);
        mounted.unmount();
        await waitFor(() => {
            expect(stored.get(persistedKeys.dateGridVisibility)).toEqual({ hideFrequentRecurring: false, hideTimed: true, hideDoneTasks: false });
        });
        render(DateGrid);
        await screen.findByText('One-off all day');
        expect(screen.queryByText('One-off timed')).toBeNull();
        const restored = await openFilters();
        expect(restored.timed).toHaveProperty('checked', true);
        expect(screen.getByTitle('Filter').textContent?.trim()).toBe('2');
        expect(screen.getByRole('button', { name: /event calendar/i })).toBeTruthy();
    });

    it('ignores malformed stored visibility instead of accidentally hiding events', async () => {
        tauri.stubCommand('get_ui_state', args => args?.key === persistedKeys.dateGridVisibility
            ? { hideFrequentRecurring: 'false', hideTimed: true }
            : undefined);
        render(DateGrid);
        await screen.findByText('One-off timed');
        const { recurring, timed } = await openFilters();
        expect(recurring).toHaveProperty('checked', false);
        expect(timed).toHaveProperty('checked', false);
    });
});

describe('DateGrid done-task hiding', () => {
    beforeEach(() => {
        tauri.reset();
        const start = startOfWeek(new Date());
        const startDate = ymd(start);
        tauri.stubCommandValue('get_active_calendars', calendars);
        tauri.stubCommandValue('get_ui_state', undefined);
        tauri.stubCommandValue('query_event_instances', [
            makeInstance({ id: 'planned-done', title: 'Planned done', taskId: 't1', taskStatus: 'done', timing: { kind: 'allDay', startDate, endDateExclusive: ymd(addDays(start, 1)) } }),
            makeInstance({ id: 'planned-todo', title: 'Planned todo', taskId: 't2', taskStatus: 'todo', timing: { kind: 'allDay', startDate, endDateExclusive: ymd(addDays(start, 1)) } }),
            makeInstance({ id: 'plain', title: 'Plain event', timing: { kind: 'allDay', startDate, endDateExclusive: ymd(addDays(start, 1)) } }),
        ]);
        tauri.stubCommandValue('query_task_dates', [
            makeTaskDateInstance({ taskId: 't1', kind: 'due', title: 'Done marker', status: 'done', timing: { kind: 'allDay', date: ymd(addDays(start, 2)) } }),
            makeTaskDateInstance({ taskId: 't2', kind: 'due', title: 'Open marker', status: 'todo', timing: { kind: 'allDay', date: ymd(addDays(start, 1)) } }),
        ]);
    });

    it('hides done planned events and done task markers only after opting in', async () => {
        render(DateGrid);
        expect(await screen.findByText('Planned done')).toBeTruthy();
        expect(await screen.findByText('Done marker')).toBeTruthy();

        const { done } = await openFilters();
        await fireEvent.click(done);

        expect(screen.queryByText('Planned done')).toBeNull();
        expect(screen.queryByText('Done marker')).toBeNull();
        expect(screen.getByText('Planned todo')).toBeTruthy();
        expect(screen.getByText('Plain event')).toBeTruthy();
        expect(screen.getByText('Open marker')).toBeTruthy();
        expect(screen.getByTitle('Filter').textContent?.trim()).toBe('1');
    });
});

describe('DateGrid event titles', () => {
    beforeEach(() => {
        tauri.reset();
        const start = startOfWeek(new Date());
        tauri.stubCommandValue('get_active_calendars', calendars);
        tauri.stubCommandValue('get_ui_state', undefined);
        tauri.stubCommandValue('query_event_instances', [
            makeInstance({
                id: 'unnamed',
                title: '',
                timing: { kind: 'allDay', startDate: ymd(start), endDateExclusive: ymd(addDays(start, 1)) },
            }),
        ]);
    });

    it('labels an unnamed event instead of rendering an empty box', async () => {
        render(DateGrid);
        expect(await screen.findByText('Untitled')).toBeTruthy();
    });
});

describe('DateGrid task dates', () => {
    beforeEach(() => {
        tauri.reset();
        const start = startOfWeek(new Date());
        const startDate = ymd(start);
        tauri.stubCommandValue('get_active_calendars', calendars);
        tauri.stubCommandValue('get_ui_state', undefined);
        tauri.stubCommandValue('query_event_instances', []);
        tauri.stubCommandValue('query_task_dates', [
            makeTaskDateInstance({
                taskId: 'rent',
                kind: 'start',
                title: 'Pay rent',
                timing: { kind: 'allDay', date: startDate },
            }),
            makeTaskDateInstance({
                taskId: 'rent',
                kind: 'due',
                title: 'Pay rent',
                timing: { kind: 'allDay', date: startDate },
            }),
            makeTaskDateInstance({
                taskId: 'call',
                kind: 'due',
                title: 'Call mom',
                timing: { kind: 'allDay', date: ymd(addDays(start, 1)) },
            }),
        ]);
    });

    it('renders a start and due marker even when they land on the same day', async () => {
        const onTaskClick = vi.fn();
        render(DateGrid, { onTaskClick });

        expect(await screen.findAllByText('Pay rent')).toHaveLength(2);

        expect(screen.getByText('Call mom')).toBeTruthy();
    });

    it('opens the task when a marker is clicked', async () => {
        const onTaskClick = vi.fn();
        render(DateGrid, { onTaskClick });

        await fireEvent.click((await screen.findAllByText('Pay rent'))[0]);

        expect(onTaskClick).toHaveBeenCalledTimes(1);
        expect(onTaskClick.mock.calls[0][0]).toMatchObject({ taskId: 'rent', kind: 'start' });
    });

    it('starts dragging on writable marker body, ignores circle pointerdown, and blocks read-only markers', async () => {
        const start = startOfWeek(new Date());
        const startDate = ymd(start);
        const onTaskDragStart = vi.fn();
        const onTaskClick = vi.fn();
        const onToggleTask = vi.fn();

        tauri.stubCommandValue('get_active_calendars', [
            ...calendars,
            { id: 'ro-tasks', summary: 'Shared Tasks', writable: false, component: 'VTODO' },
        ]);
        tauri.stubCommandValue('query_task_dates', [
            makeTaskDateInstance({
                taskId: 'writable-task',
                kind: 'start',
                title: 'Editable task',
                calendarId: 'tasks',
                timing: { kind: 'allDay', date: startDate },
            }),
            makeTaskDateInstance({
                taskId: 'readonly-task',
                kind: 'start',
                title: 'Readonly task',
                calendarId: 'ro-tasks',
                timing: { kind: 'allDay', date: startDate },
            }),
        ]);

        render(DateGrid, { onTaskDragStart, onTaskClick, onToggleTask });

        const editable = (await screen.findByText('Editable task')).closest('.day-cell-task') as HTMLElement;
        const readonly = screen.getByText('Readonly task').closest('.day-cell-task') as HTMLElement;

        expect(editable.style.cursor).toBe('grab');
        expect(readonly.classList.contains('is-readonly')).toBe(true);
        expect(readonly.style.cursor).toBe('default');

        const circle = editable.querySelector('.task-circle') as HTMLElement;
        await fireEvent.pointerDown(circle, { button: 0 });
        await fireEvent.click(circle);
        expect(onTaskDragStart).not.toHaveBeenCalled();
        expect(onTaskClick).not.toHaveBeenCalled();
        expect(onToggleTask).toHaveBeenCalledWith('writable-task');

        await fireEvent.pointerDown(readonly, { button: 0 });
        expect(onTaskDragStart).not.toHaveBeenCalled();

        await fireEvent.pointerDown(editable, { button: 0 });
        expect(onTaskDragStart).toHaveBeenCalledTimes(1);
        expect(onTaskDragStart.mock.calls[0][1]).toMatchObject({ taskId: 'writable-task', kind: 'start' });
    });

    it('fades only the dragged marker occurrence and shows kind-specific drop hint text', async () => {
        const start = startOfWeek(new Date());
        const targetDate = ymd(addDays(start, 2));

        const { container, rerender } = render(DateGrid, {
            dragState: {
                task: { id: 'rent', markerKind: 'due', occurrenceKey: 'rent@due' },
                target: { kind: 'grid-day', date: targetDate },
            },
        });

        await screen.findAllByText('Pay rent');
        const capsules = Array.from(container.querySelectorAll<HTMLElement>('.day-cell-task'));
        const startCapsule = capsules.find((el) => el.getAttribute('title')?.startsWith('Start: Pay rent'));
        const dueCapsule = capsules.find((el) => el.getAttribute('title')?.startsWith('Due: Pay rent'));

        expect(startCapsule?.classList.contains('is-dragging')).toBe(false);
        expect(dueCapsule?.classList.contains('is-dragging')).toBe(true);
        expect(dueCapsule?.style.opacity).toBe('0.4');
        expect(screen.getByText('drop to move due')).toBeTruthy();

        await rerender({
            dragState: {
                task: { id: 'rent', markerKind: 'start', occurrenceKey: 'rent@start' },
                target: { kind: 'grid-day', date: targetDate },
            },
        });
        expect(screen.getByText('drop to move start')).toBeTruthy();
    });
});

describe('DateGrid vertical infinite scroll', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('get_active_calendars', calendars);
        tauri.stubCommandValue('query_event_instances', []);
        tauri.stubCommandValue('query_task_dates', []);
    });

    it('highlights the most-shown month as active while scrolling down in month view', async () => {
        tauri.stubCommand('get_ui_state', (args) =>
            args?.key === persistedKeys.dateGridView ? 'month' : undefined,
        );
        const host = document.createElement('div');
        host.style.width = '700px';
        host.style.height = '660px';
        document.body.appendChild(host);

        try {
            const { container } = render(DateGrid, { target: host });
            const scroller = container.querySelector('.cal-cells') as HTMLDivElement;

            await waitFor(() => {
                expect(scroller.classList.contains('has-week-height')).toBe(true);
                expect(scroller.scrollTop).toBeGreaterThan(0);
            });

            const initialTitle = container.querySelector('.cal-hd-title')?.textContent?.trim();
            const weekHeight = scroller.clientHeight / 6;

            // Scroll down 5 weeks (~1 full month forward) so the next month dominates the viewport.
            scroller.scrollTop += weekHeight * 5;
            scroller.dispatchEvent(new Event('scroll'));

            await waitFor(() => {
                const nextTitle = container.querySelector('.cal-hd-title')?.textContent?.trim();
                expect(nextTitle).not.toBe(initialTitle);
            });

            // The dominant month's cells are active (not .is-out), while surrounding months are .is-out.
            const activeCells = container.querySelectorAll('.day-cell:not(.is-out)');
            const outCells = container.querySelectorAll('.day-cell.is-out');
            expect(activeCells.length).toBeGreaterThanOrEqual(28);
            expect(activeCells.length).toBeLessThanOrEqual(31);
            expect(outCells.length).toBeGreaterThan(0);
        } finally {
            host.remove();
        }
    });

    it('extends rendered weeks when scrolling continuously in week view', async () => {
        tauri.stubCommand('get_ui_state', (args) =>
            args?.key === persistedKeys.dateGridView ? '1 week' : undefined,
        );
        const host = document.createElement('div');
        host.style.width = '700px';
        host.style.height = '320px';
        document.body.appendChild(host);

        try {
            const { container } = render(DateGrid, { target: host });
            const scroller = container.querySelector('.cal-cells') as HTMLDivElement;

            await waitFor(() => {
                expect(scroller.classList.contains('has-week-height')).toBe(true);
                expect(scroller.scrollTop).toBeGreaterThan(0);
            });

            const farFutureWeek = ymd(addDays(startOfWeek(new Date()), 7 * 12));
            expect(container.querySelector(`[data-week-start="${farFutureWeek}"]`)).toBeNull();

            scroller.scrollTop = scroller.scrollHeight - scroller.clientHeight;
            scroller.dispatchEvent(new Event('scroll'));

            await waitFor(() => {
                expect(container.querySelector(`[data-week-start="${farFutureWeek}"]`)).not.toBeNull();
            });
        } finally {
            host.remove();
        }
    });

    it('keeps inner day cells non-scrollable (overflow: clip) and aligns arrow navigation flush to the top row', async () => {
        tauri.stubCommand('get_ui_state', (args) =>
            args?.key === persistedKeys.dateGridView ? 'month' : undefined,
        );
        const host = document.createElement('div');
        host.style.width = '700px';
        host.style.height = '653px'; // intentionally non-divisible by 6 to test fractional row alignment
        document.body.appendChild(host);

        try {
            const { container } = render(DateGrid, { target: host });
            const scroller = container.querySelector('.cal-cells') as HTMLDivElement;

            await waitFor(() => {
                expect(scroller.classList.contains('has-week-height')).toBe(true);
                expect(scroller.scrollTop).toBeGreaterThan(0);
            });

            // Inner day cells must use clip (not hidden) so overscroll-behavior: none cannot swallow wheel events.
            const firstCell = container.querySelector('.day-cell') as HTMLElement;
            const firstEvents = container.querySelector('.day-cell-events-scroll') as HTMLElement;
            expect(getComputedStyle(firstCell).overflowY).toBe('clip');
            expect(getComputedStyle(firstEvents).overflowY).toBe('clip');

            // Clicking Next month and waiting for scrollend leaves the top visible week flush with the viewport top.
            const nextBtn = screen.getByRole('button', { name: 'next' });
            await fireEvent.click(nextBtn);

            await waitFor(() => {
                const scrollerTop = scroller.getBoundingClientRect().top + 1; // +1px border-top
                const weeks = Array.from(container.querySelectorAll<HTMLElement>('.cal-week'));
                const topVisible = weeks.find(
                    (w) => Math.abs(w.getBoundingClientRect().top - scrollerTop) <= 1.5,
                );
                expect(topVisible).toBeDefined();
            });
        } finally {
            host.remove();
        }
    });

    it('keeps all 7 day cells equal width even with long event titles and snaps vertically onto week rows', async () => {
        const start = startOfWeek(new Date());
        tauri.stubCommand('get_ui_state', (args) =>
            args?.key === persistedKeys.dateGridView ? 'month' : undefined,
        );
        tauri.stubCommandValue('query_event_instances', [
            makeInstance({
                id: 'long-title',
                title: 'An extraordinarily long single-day event title that must never widen its grid column',
                timing: { kind: 'allDay', startDate: ymd(start), endDateExclusive: ymd(addDays(start, 1)) },
            }),
        ]);

        const host = document.createElement('div');
        host.style.width = '732px';
        host.style.height = '660px';
        document.body.appendChild(host);

        try {
            const { container } = render(DateGrid, { target: host });
            const scroller = container.querySelector('.cal-cells') as HTMLDivElement;

            await screen.findByText(/extraordinarily long/);
            await waitFor(() => {
                expect(scroller.classList.contains('has-week-height')).toBe(true);
                expect(scroller.scrollTop).toBeGreaterThan(0);
            });

            // Every cell in the week containing the long title must share the exact same width.
            const targetWeek = container.querySelector(`[data-week-start="${ymd(start)}"]`) as HTMLElement;
            const widths = Array.from(targetWeek.querySelectorAll<HTMLElement>('.day-cell'), (c) =>
                c.getBoundingClientRect().width,
            );
            expect(widths).toHaveLength(7);
            for (const width of widths) {
                expect(width).toBeCloseTo(widths[0], 0);
            }

            // Stopping partway down a week smoothly snaps onto the nearest week row boundary.
            const initialTop = scroller.scrollTop;
            const weekHeight = scroller.clientHeight / 6;
            const scrollCalls: ScrollToOptions[] = [];
            scroller.scrollTo = ((arg?: ScrollToOptions | number) => {
                if (typeof arg === 'object' && arg !== null) scrollCalls.push(arg);
            }) as typeof scroller.scrollTo;
            scroller.scrollTop = initialTop + weekHeight * 0.7;
            scroller.dispatchEvent(new Event('scrollend'));

            expect(scrollCalls).toHaveLength(1);
            expect(scrollCalls[0]?.behavior).toBe('smooth');
            expect(Math.abs((scrollCalls[0]?.top ?? 0) - (initialTop + weekHeight))).toBeLessThanOrEqual(1);
        } finally {
            host.remove();
        }
    });
});
