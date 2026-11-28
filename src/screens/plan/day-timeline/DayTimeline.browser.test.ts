import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { filterGroup } from '../../../lib/domain';
import type { AppEventFilter, FilterGroup } from '../../../lib/domain';
import { persistedKeys } from '../../../lib/persisted.svelte';
import { store } from '../../../lib/store.svelte';
import { addDays, ymd } from '../../../lib/time';
import { makeInstance } from '../../../test/factories';
import { mockTauriInvoke } from '../../../test/tauriMock';
import { EVENT_INSET_PX, PIXELS_PER_HOUR, PX_PER_MIN } from './constants';
import DayTimeline from './DayTimeline.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

describe('DayTimeline infinite horizontal scroll', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('query_event_instances', []);
        tauri.stubCommandValue('get_active_calendars', []);
        tauri.stubCommand('get_ui_state', (args) =>
            args?.key === 'ui.day_timeline.num_days' ? 3 : undefined,
        );
    });

    it('marks only today in the buffered header strip and derives column width from widget width', async () => {
        const host = document.createElement('div');
        host.style.width = '676px'; // 32px pane padding + 44px gutter + 3 * 200px days
        host.style.height = '400px';
        document.body.appendChild(host);

        try {
            const { container } = render(DayTimeline, { target: host });
            const todayKey = ymd(new Date());

            await waitFor(() => {
                const todayCells = container.querySelectorAll('.day-head-cell.is-today');
                expect(todayCells).toHaveLength(1);
                expect(todayCells[0]?.getAttribute('data-day')).toBe(todayKey);
                const firstGrid = container.querySelector('.day-grid') as HTMLElement | null;
                expect(firstGrid?.getBoundingClientRect().width).toBeCloseTo(200, 0);
            });

            // Buffer days exist on both sides of today so scrolling works in either direction.
            const dayKeys = Array.from(container.querySelectorAll('.day-head-cell'), (el) =>
                el.getAttribute('data-day'),
            );
            const todayIndex = dayKeys.indexOf(todayKey);
            expect(todayIndex).toBeGreaterThan(0);
            expect(todayIndex).toBeLessThan(dayKeys.length - 1);
        } finally {
            host.remove();
        }
    });

    it('smoothly snaps to a day boundary on scrollend and loads further days when scrolling continuously', async () => {
        const host = document.createElement('div');
        host.style.width = '676px'; // 32px pane padding + 44px gutter + 3 * 200px days
        host.style.height = '400px';
        document.body.appendChild(host);

        try {
            const { container } = render(DayTimeline, { target: host });
            const scroller = container.querySelector('.day-scroll') as HTMLDivElement;

            await waitFor(() => {
                const firstGrid = container.querySelector('.day-grid') as HTMLElement | null;
                expect(firstGrid?.getBoundingClientRect().width).toBeCloseTo(200, 0);
                expect(scroller.scrollLeft).toBeGreaterThan(0);
            });

            const initialScrollLeft = scroller.scrollLeft;
            const scrollToSpy = vi.spyOn(scroller, 'scrollTo');
            scroller.scrollLeft = initialScrollLeft + 130;
            scroller.dispatchEvent(new Event('scrollend'));

            expect(scrollToSpy).toHaveBeenCalledWith({
                left: initialScrollLeft + 200,
                behavior: 'smooth',
            });

            // Finish the snap gesture, then scroll far right to verify the window advances to future days.
            scroller.dispatchEvent(new Event('scrollend'));
            const futureDay = ymd(addDays(new Date(), 20));
            expect(container.querySelector(`[data-day="${futureDay}"]`)).toBeNull();

            scroller.scrollLeft = scroller.scrollWidth - scroller.clientWidth;
            scroller.dispatchEvent(new Event('scroll'));

            await waitFor(() => {
                expect(container.querySelector(`[data-day="${futureDay}"]`)).not.toBeNull();
            });
        } finally {
            host.remove();
        }
    });
});

const nestedFilters: FilterGroup<AppEventFilter> = { operator: 'and', children: [
    { kind: 'condition', filter: { column: 'remotecollectionid', operator: 'is', value: ['events'] } },
    { kind: 'group', ...filterGroup<AppEventFilter>([
        { column: 'status', operator: 'is', value: ['confirmed'] },
        { column: 'status', operator: 'is', value: ['tentative'] },
    ], 'or') },
] };

describe('DayTimeline nested filters', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('query_event_instances', []);
        tauri.stubCommandValue('get_active_calendars', []);
        tauri.stubCommandValue('list_tags', []);
    });
    it.each(['plan', 'sidebar'] as const)('restores, queries and saves a tree in the %s timeline', async variant => {
        const key = variant === 'plan' ? persistedKeys.dayTimelineFilters : persistedKeys.sidebarTimelineFilters;
        tauri.stubCommand('get_ui_state', args => args?.key === key ? nestedFilters : undefined);
        const mounted = render(DayTimeline, { props: { variant } });
        await waitFor(() => { expect(tauri.invoke).toHaveBeenCalledWith('query_event_instances', expect.objectContaining({ filters: nestedFilters }), undefined); });
        await fireEvent.click(screen.getByRole('button', { name: 'Filter' }));
        expect(screen.getAllByRole('combobox', { name: 'Group operator' })).toHaveLength(2);
        await fireEvent.change(screen.getAllByRole('combobox', { name: 'Group operator' })[0]!, { target: { value: 'or' } });
        mounted.unmount();
        await waitFor(() => { expect(tauri.invoke).toHaveBeenCalledWith('set_ui_state', { key, value: { ...nestedFilters, operator: 'or' } }, undefined); });
    });
});

describe('DayTimeline earliest_wake_up_time', () => {
    beforeEach(() => {
        tauri.reset();
        store.settings = null;
        tauri.stubCommandValue('query_event_instances', []);
        tauri.stubCommandValue('get_active_calendars', []);
        tauri.stubCommandValue('list_tags', []);
        tauri.stubCommandValue('get_ui_state', undefined);
    });

    afterEach(() => {
        cleanup();
        vi.useRealTimers();
        store.settings = null;
    });

    it('renders all 24 hours starting at 00:00 by default', () => {
        const { container } = render(DayTimeline);

        const labels = Array.from(
            container.querySelectorAll<HTMLElement>('.day-hour-label'),
            (el) => ({ text: el.textContent?.trim(), top: Number.parseFloat(el.style.top) }),
        );
        expect(labels).toHaveLength(24);
        expect(labels[0]).toEqual({ text: '00:00', top: 0 });
        expect(labels[23]).toEqual({ text: '23:00', top: 23 * PIXELS_PER_HOUR });

        const grid = container.querySelector<HTMLElement>('.day-grid')!;
        expect(Number.parseFloat(grid.style.height)).toBeCloseTo(24 * PIXELS_PER_HOUR, 1);
        expect(grid.getAttribute('data-start-minute')).toBe('0');
    });

    it('hides pre-wake-up hours, offsets events/creation, and adjusts initial scrollTop when set to 06:00', async () => {
        vi.useFakeTimers({ toFake: ['Date'] });
        vi.setSystemTime(new Date(2026, 8, 7, 10, 0));

        const now = new Date();
        const todayStart = (h: number, m = 0) =>
            new Date(now.getFullYear(), now.getMonth(), now.getDate(), h, m).toISOString();

        store.settings = { earliest_wake_up_time: 360 };
        tauri.stubCommandValue('query_event_instances', [
            makeInstance({
                id: 'ev-early',
                title: 'Deep Sleep',
                timing: { kind: 'timed', start: todayStart(4, 0), end: todayStart(5, 30), timezone: null },
            }),
            makeInstance({
                id: 'ev-morning',
                title: 'Morning Review',
                timing: { kind: 'timed', start: todayStart(9, 0), end: todayStart(10, 0), timezone: null },
            }),
        ]);

        const onAddEvent = vi.fn(() => Promise.resolve(undefined));
        const host = document.createElement('div');
        host.style.width = '400px';
        host.style.height = '300px';
        document.body.appendChild(host);

        try {
            const { container } = render(DayTimeline, { target: host, props: { onAddEvent } });

            const labels = Array.from(
                container.querySelectorAll<HTMLElement>('.day-hour-label'),
                (el) => ({ text: el.textContent?.trim(), top: Number.parseFloat(el.style.top) }),
            );
            expect(labels).toHaveLength(18);
            expect(labels[0]).toEqual({ text: '06:00', top: 0 });
            expect(labels[17]).toEqual({ text: '23:00', top: 17 * PIXELS_PER_HOUR });
            for (const hidden of ['00:00', '01:00', '02:00', '03:00', '04:00', '05:00']) {
                expect(labels.some((l) => l.text === hidden)).toBe(false);
            }

            const scroller = container.querySelector<HTMLDivElement>('.day-scroll')!;
            // 10:00 (600m) - 120m lead - 360m wake-up = 120m
            expect(scroller.scrollTop).toBeCloseTo(120 * PX_PER_MIN, 0);

            await waitFor(() => {
                expect(screen.getByText('Morning Review')).toBeTruthy();
                expect(container.querySelector('.day-now-label')).not.toBeNull();
            });
            const nowLabel = container.querySelector<HTMLElement>('.day-now-label')!;
            expect(nowLabel.textContent?.trim()).toBe('10:00');
            expect(Number.parseFloat(nowLabel.style.top)).toBeCloseTo(4 * PIXELS_PER_HOUR, 1);
            expect(screen.queryByText('Deep Sleep')).toBeNull();

            const block = container.querySelector<HTMLElement>('.day-event')!;
            expect(Number.parseFloat(block.style.top)).toBeCloseTo(
                3 * PIXELS_PER_HOUR + EVENT_INSET_PX,
                1,
            );

            const todayCol = container.querySelector<HTMLElement>(
                `[data-drop-kind="day-time"][data-drop-date="${ymd(now)}"]`,
            )!;
            expect(Number.parseFloat(todayCol.style.height)).toBeCloseTo(18 * PIXELS_PER_HOUR, 1);
            expect(todayCol.getAttribute('data-start-minute')).toBe('360');

            const rect = todayCol.getBoundingClientRect();
            await fireEvent.pointerDown(todayCol, { button: 0, clientY: rect.top + PIXELS_PER_HOUR });
            await fireEvent.pointerUp(window, { clientY: rect.top + PIXELS_PER_HOUR });
            expect(onAddEvent).toHaveBeenCalledWith(expect.any(Date), 7 * 60, 8 * 60);
        } finally {
            host.remove();
        }
    });

    it('hides now indicators in the gutter and column when current time is before wake-up time', async () => {
        vi.useFakeTimers({ toFake: ['Date'] });
        vi.setSystemTime(new Date(2026, 8, 7, 5, 55));
        store.settings = { earliest_wake_up_time: 360 };

        const { container } = render(DayTimeline);
        await waitFor(() => {
            expect(container.querySelector('.day-hour-label')?.textContent?.trim()).toBe('06:00');
        });

        expect(container.querySelector('.day-now-label')).toBeNull();
        expect(container.querySelector('.day-now')).toBeNull();
        expect(container.querySelector<HTMLElement>('.day-hour-label')!.style.opacity).toBe('1');
    });
});
