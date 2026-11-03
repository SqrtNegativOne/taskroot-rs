import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../../test/tauriMock';
import DayTimeline from './DayTimeline.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

const DAY_HEADER_PATTERN = /^[A-Z][a-z]{2}, [A-Z][a-z]{2} \d{1,2}$/;

function rect(left: number): DOMRect {
    return {
        left,
        right: left,
        top: 0,
        bottom: 0,
        width: 0,
        height: 0,
        x: left,
        y: 0,
        toJSON: () => ({}),
    };
}

/** jsdom has no layout, so hand the scroller and its columns scroll geometry. */
function stubGeometry(scroller: HTMLElement): void {
    Object.defineProperty(scroller, 'scrollWidth', { value: 1000, configurable: true });
    Object.defineProperty(scroller, 'clientWidth', { value: 500, configurable: true });
    scroller.getBoundingClientRect = () => rect(0);
    scroller.querySelectorAll<HTMLElement>('.day-grid').forEach((column, index) => {
        column.getBoundingClientRect = () => rect(16 + index * 200);
    });
}

describe('DayTimeline day headers', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('query_event_instances', []);
        tauri.stubCommandValue('get_active_calendars', []);
        tauri.stubCommand('get_ui_state', (args) =>
            args?.key === 'ui.day_timeline.num_days' ? 3 : undefined,
        );
    });

    afterEach(() => {
        cleanup();
    });

    it('names every visible day column in a header cell directly above it', async () => {
        const { container } = render(DayTimeline);

        await waitFor(() => {
            expect(container.querySelectorAll('.day-head-cell')).toHaveLength(3);
        });
        const cells = container.querySelectorAll('.day-head-cell');
        const columns = container.querySelectorAll('.day-grid');
        expect(columns).toHaveLength(3);

        cells.forEach((cell, index) => {
            expect(cell.getAttribute('data-day')).toBe(columns[index].getAttribute('data-drop-date'));
            expect(cell.textContent).toMatch(DAY_HEADER_PATTERN);
        });
    });

    it('marks the header of today like the timeline does', async () => {
        const { container } = render(DayTimeline);

        await waitFor(() => {
            expect(container.querySelectorAll('.day-head-cell')).toHaveLength(3);
            expect(container.querySelector('.day-head-cell.is-today')).not.toBeNull();
        });
        const cells = container.querySelectorAll('.day-head-cell');
        expect(cells[0].classList.contains('is-today')).toBe(true);
        expect(cells[1].classList.contains('is-today')).toBe(false);
    });

    it('aligns the horizontal scroll to a day boundary when a scroll ends', async () => {
        const { container } = render(DayTimeline);
        await waitFor(() => {
            expect(container.querySelectorAll('.day-grid')).toHaveLength(3);
        });
        const scroller = container.querySelector('.day-scroll') as HTMLDivElement;
        stubGeometry(scroller);

        scroller.scrollLeft = 130;
        scroller.dispatchEvent(new Event('scrollend'));

        expect(scroller.scrollLeft).toBe(200);
    });
});
