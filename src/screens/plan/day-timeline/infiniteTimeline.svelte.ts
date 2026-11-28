import { untrack } from 'svelte';
import { addDays, dayDiff, ymd } from '../../../lib/time';
import { startOfLocalDay, windowLength, type InfiniteWindowConfig } from '../infiniteScroll';
import { createInfiniteWindowController } from '../infiniteScroll.svelte';

const DEFAULT_GUTTER_WIDTH_PX = 44;
const MIN_COLUMN_WIDTH_PX = 40;
const EMERGENCY_MARGIN_DAYS = 2;
const SETTLE_DRIFT_DAYS = 3;

function timelineWindowConfig(numDays: number): InfiniteWindowConfig {
    const visibleCount = Math.max(1, numDays);
    const bufferCount = Math.max(14, visibleCount * 2 + 6);
    return { visibleCount, bufferCount, recenterMargin: EMERGENCY_MARGIN_DAYS };
}

function timelineColumnWidth(viewportWidth: number, gutterWidth: number, numDays: number): number {
    if (viewportWidth <= gutterWidth || numDays <= 0) return 0;
    return Math.max(MIN_COLUMN_WIDTH_PX, (viewportWidth - gutterWidth) / numDays);
}

function measureGutterWidth(scroller: HTMLElement): number {
    const measured = scroller.querySelector<HTMLElement>('.day-gutter')?.getBoundingClientRect().width ?? 0;
    return measured > 0 ? measured : DEFAULT_GUTTER_WIDTH_PX;
}

function findDayColumn(scroller: HTMLElement, date: Date): HTMLElement | null {
    return scroller.querySelector<HTMLElement>(`.day-grid[data-drop-date="${ymd(date)}"]`);
}

function buildWindowDates(windowStart: Date, config: InfiniteWindowConfig): Date[] {
    return Array.from({ length: windowLength(config) }, (_, i) => addDays(windowStart, i));
}

/**
 * Horizontal infinite-scroll controller for `DayTimeline`. Keeps a buffered
 * sliding window of day columns centered around `viewDate` and derives each
 * column's width from the available viewport divided by `numDays`.
 */
export function createInfiniteTimeline(
    getNumDays: () => number,
    initialDate: Date = new Date(),
) {
    const initialDay = startOfLocalDay(initialDate);
    let viewDate = $state(initialDay);
    let gutterWidth = $state(DEFAULT_GUTTER_WIDTH_PX);
    const readViewDate = () => viewDate;

    const window = createInfiniteWindowController({
        axis: 'left',
        stepDays: 1,
        settleDrift: SETTLE_DRIFT_DAYS,
        initialTargetItem: initialDay,
        firstItemSelector: '.day-grid',
        findItemElement: findDayColumn,
        getConfig: () => timelineWindowConfig(getNumDays()),
        getItemSize: (viewportWidth, cfg) =>
            timelineColumnWidth(viewportWidth, gutterWidth, cfg.visibleCount),
        getTargetItem: () => untrack(readViewDate),
        onSyncAnchor: (leadingIndex) => {
            const visible = addDays(window.windowStart, leadingIndex);
            if (dayDiff(visible, viewDate) !== 0) {
                viewDate = visible;
            }
        },
        onMeasure: (el) => { gutterWidth = measureGutterWidth(el); },
    });

    const dates = $derived(buildWindowDates(window.windowStart, window.config));
    const dateRange = $derived({
        startDate: ymd(dates[0]),
        endDate: ymd(dates[dates.length - 1]),
    });

    return {
        get viewDate() { return viewDate; },
        get config() { return window.config; },
        get colWidth() { return window.itemSize; },
        get dates() { return dates; },
        get dateRange() { return dateRange; },
        settle: window.settle,
        beginSnap: window.beginSnap,
        attach: window.attach,
        goToDate(target: Date, smooth = true): void {
            const normalized = startOfLocalDay(target);
            viewDate = normalized;
            window.scrollToItem(normalized, smooth);
        },
    };
}
