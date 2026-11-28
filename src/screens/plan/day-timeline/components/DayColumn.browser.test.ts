import { tick } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import DayColumn from './DayColumn.svelte';
import { EVENT_INSET_PX, PIXELS_PER_HOUR, PX_PER_MIN } from '../constants';
import { targetFromElement } from '../../date-grid/dropTarget';
import { makeInstance } from '../../../../test/factories';
import type { LaidEvent } from '../types';

const date = new Date(2026, 8, 7);
const today = new Date(2026, 8, 8);

function laidFor(id: string, title: string, startMins = 60, endMins = 120): LaidEvent {
    return {
        event: makeInstance({ id, title }),
        startMins,
        endMins,
        lane: 0,
        lanes: 1,
    };
}

async function pressAndRelease(grid: Element, clientY = 0) {
    await fireEvent.pointerDown(grid, { button: 0, clientY });
    await fireEvent.pointerUp(window, { clientY });
}

describe('DayColumn creation ghost', () => {
    afterEach(() => {
        cleanup();
        vi.useRealTimers();
    });

    it('shows the ghost as soon as the grid is pressed, before any movement', async () => {
        const { container } = render(DayColumn, {
            props: { date, today, laid: [], onAddEvent: vi.fn() },
        });
        const grid = container.querySelector('.day-grid');
        expect(grid).not.toBeNull();
        expect(screen.queryByText('New Event...')).toBeNull();

        await fireEvent.pointerDown(grid!, { button: 0, clientY: 0 });

        expect(screen.getByText('New Event...')).toBeTruthy();
    });

    it('keeps the ghost after release until the created event reaches the layout', async () => {
        const onAddEvent = vi.fn(() => Promise.resolve('created-1'));
        const { container, rerender } = render(DayColumn, {
            props: { date, today, laid: [], onAddEvent },
        });
        const grid = container.querySelector('.day-grid');
        expect(grid).not.toBeNull();

        await pressAndRelease(grid!);

        expect(onAddEvent).toHaveBeenCalledWith(date, 0, 60);
        await new Promise((resolve) => setTimeout(resolve, 0));
        expect(screen.getByText('New Event...')).toBeTruthy();

        await rerender({ laid: [laidFor('created-1', 'Created')] });

        await waitFor(() => expect(screen.queryByText('New Event...')).toBeNull());
        expect(screen.getByText('Created')).toBeTruthy();
    });

    it('drops the ghost when creation fails', async () => {
        const onAddEvent = vi.fn(() => Promise.resolve(undefined));
        const { container } = render(DayColumn, {
            props: { date, today, laid: [], onAddEvent },
        });
        const grid = container.querySelector('.day-grid');
        expect(grid).not.toBeNull();

        await pressAndRelease(grid!);
        await new Promise((resolve) => setTimeout(resolve, 0));

        expect(screen.queryByText('New Event...')).toBeNull();
    });
});

describe('DayColumn startHour offset', () => {
    afterEach(() => {
        cleanup();
        vi.useRealTimers();
    });

    it('sizes the grid, hides pre-wake events, and offsets creation and drop targets from startHour', async () => {
        const onAddEvent = vi.fn(() => Promise.resolve(undefined));
        const laid: LaidEvent[] = [
            laidFor('before', 'Asleep', 2 * 60, 6 * 60),
            laidFor('spanning', 'Early Bird', 5 * 60, 7 * 60),
            laidFor('morning', 'Standup', 9 * 60, 10 * 60),
        ];

        const { container } = render(DayColumn, {
            props: {
                date,
                today,
                laid,
                startHour: 6,
                onAddEvent,
                dragState: {
                    target: { kind: 'day-time', date: '2026-09-07', minute: 7 * 60, duration: 60 },
                },
            },
        });

        const grid = container.querySelector<HTMLElement>('.day-grid')!;
        expect(Number.parseFloat(grid.style.height)).toBeCloseTo(18 * PIXELS_PER_HOUR, 1);
        expect(grid.getAttribute('data-start-minute')).toBe('360');
        expect(container.querySelectorAll('.day-hour')).toHaveLength(18);

        expect(screen.queryByText('Asleep')).toBeNull();
        expect(screen.getByText('Early Bird')).toBeTruthy();
        expect(screen.getByText('05:00 – 07:00')).toBeTruthy();
        expect(screen.getByText('Standup')).toBeTruthy();

        const blocks = container.querySelectorAll<HTMLElement>('.day-event');
        expect(Number.parseFloat(blocks[0]!.style.top)).toBeCloseTo(EVENT_INSET_PX, 1);
        expect(Number.parseFloat(blocks[1]!.style.top)).toBeCloseTo(
            3 * PIXELS_PER_HOUR + EVENT_INSET_PX,
            1,
        );

        const dropPreview = container.querySelector<HTMLElement>('.day-drop-preview')!;
        expect(Number.parseFloat(dropPreview.style.top)).toBeCloseTo(PIXELS_PER_HOUR, 1);
        expect(dropPreview.textContent).toContain('07:00 – 08:00');

        const rect = grid.getBoundingClientRect();
        expect(targetFromElement(grid, rect.top + PIXELS_PER_HOUR, 60)).toEqual({
            kind: 'day-time',
            date: '2026-09-07',
            minute: 7 * 60,
            duration: 60,
        });
        expect(targetFromElement(grid, rect.top - 30, 60)).toEqual({
            kind: 'day-time',
            date: '2026-09-07',
            minute: 6 * 60,
            duration: 60,
        });
        expect(targetFromElement(grid, rect.top + 18 * PIXELS_PER_HOUR, 60)).toEqual({
            kind: 'day-time',
            date: '2026-09-07',
            minute: 23 * 60,
            duration: 60,
        });

        await fireEvent.pointerDown(grid, { button: 0, clientY: rect.top + PIXELS_PER_HOUR });
        const ghost = container.querySelector<HTMLElement>('.day-event.ev-plan')!;
        expect(Number.parseFloat(ghost.style.top)).toBeCloseTo(PIXELS_PER_HOUR, 1);
        expect(ghost.textContent).toContain('07:00 – 08:00');
        await fireEvent.pointerUp(window, { clientY: rect.top + PIXELS_PER_HOUR });
        expect(onAddEvent).toHaveBeenCalledWith(date, 7 * 60, 8 * 60);
    });

    it('hides CurrentTimeLine before startMinute and offsets it after startMinute', async () => {
        vi.useFakeTimers({ toFake: ['Date'] });
        vi.setSystemTime(new Date(2026, 8, 7, 5, 30));

        const early = render(DayColumn, {
            props: { date, today: date, laid: [], startHour: 6 },
        });
        await tick();
        expect(early.container.querySelector('.day-now')).toBeNull();
        cleanup();

        vi.setSystemTime(new Date(2026, 8, 7, 7, 30));
        const awake = render(DayColumn, {
            props: { date, today: date, laid: [], startHour: 6 },
        });
        await waitFor(() => {
            expect(awake.container.querySelector('.day-now')).not.toBeNull();
        });
        const nowLine = awake.container.querySelector<HTMLElement>('.day-now')!;
        expect(Number.parseFloat(nowLine.style.top)).toBeCloseTo(90 * PX_PER_MIN, 1);
    });
});
