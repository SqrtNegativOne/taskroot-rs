import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import DayColumn from './DayColumn.svelte';
import { makeInstance } from '../../../../test/factories';
import type { LaidEvent } from '../types';

const date = new Date(2026, 8, 7);
const today = new Date(2026, 8, 8);

function laidFor(id: string, title: string): LaidEvent[] {
    return [
        {
            event: makeInstance({ id, title }),
            startMins: 60,
            endMins: 120,
            lane: 0,
            lanes: 1,
        },
    ];
}

async function pressAndRelease(grid: Element) {
    await fireEvent.pointerDown(grid, { button: 0, clientY: 0 });
    await fireEvent.pointerUp(window);
}

describe('DayColumn creation ghost', () => {
    afterEach(() => {
        cleanup();
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

        await rerender({ laid: laidFor('created-1', 'Created') });

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
