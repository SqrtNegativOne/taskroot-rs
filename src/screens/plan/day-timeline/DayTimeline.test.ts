import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { filterGroup } from '../../../lib/domain';
import type { AppEventFilter, FilterGroup } from '../../../lib/domain';
import { persistedKeys } from '../../../lib/persisted.svelte';
import { mockTauriInvoke } from '../../../test/tauriMock';
import DayTimeline from './DayTimeline.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

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
        await waitFor(() => { expect(tauri.invoke).toHaveBeenCalledWith('query_event_instances', expect.objectContaining({ filters: nestedFilters })); });
        await fireEvent.click(screen.getByRole('button', { name: 'Filter' }));
        expect(screen.getAllByRole('combobox', { name: 'Group operator' })).toHaveLength(2);
        await fireEvent.change(screen.getAllByRole('combobox', { name: 'Group operator' })[0]!, { target: { value: 'or' } });
        mounted.unmount();
        await waitFor(() => { expect(tauri.invoke).toHaveBeenCalledWith('set_ui_state', { key, value: { ...nestedFilters, operator: 'or' } }); });
    });
});
