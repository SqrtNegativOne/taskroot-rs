import { untrack } from 'svelte';
import { dayDiff, startOfWeek, ymd } from '../../../lib/time';
import { DateGridView, DAYS_IN_WEEK } from './constants';
import {
    buildInfiniteWeeks,
    dominantMonthInViewport,
    monthKeyOf,
    sameMonthKey,
    shiftDateGridAnchor,
    tagOutOfMonthWeeks,
    targetWeekForAnchor,
    VISIBLE_WEEKS,
    type MonthKey,
} from './dates';
import { startOfLocalDay, windowLength, type InfiniteWindowConfig } from '../infiniteScroll';
import { createInfiniteWindowController } from '../infiniteScroll.svelte';

const EMERGENCY_MARGIN_WEEKS = 2;
const SETTLE_DRIFT_WEEKS = 2;

function gridWindowConfig(view: DateGridView): InfiniteWindowConfig {
    const visibleCount = VISIBLE_WEEKS[view];
    const bufferCount = Math.max(8, visibleCount + 4);
    return { visibleCount, bufferCount, recenterMargin: EMERGENCY_MARGIN_WEEKS };
}

function findWeekRow(scroller: HTMLElement, weekStart: Date): HTMLElement | null {
    return scroller.querySelector<HTMLElement>(`[data-week-start="${ymd(weekStart)}"]`);
}

/**
 * Vertical infinite-scroll controller for `DateGrid`. Keeps a buffered sliding
 * window of week rows, sizes each week row from the viewport height divided by
 * the view's visible week count, and tracks the most-visible calendar month.
 */
export function createInfiniteDateGrid(
    getView: () => DateGridView,
    initialDate: Date = new Date(),
) {
    const initialDay = startOfLocalDay(initialDate);
    let anchor = $state(initialDay);
    const readAnchor = () => anchor;

    const window = createInfiniteWindowController({
        axis: 'top',
        stepDays: DAYS_IN_WEEK,
        settleDrift: SETTLE_DRIFT_WEEKS,
        initialTargetItem: targetWeekForAnchor(initialDay, untrack(getView)),
        firstItemSelector: '.cal-week',
        findItemElement: findWeekRow,
        getConfig: () => gridWindowConfig(getView()),
        getItemSize: (viewportHeight, cfg) => (viewportHeight > 0 ? viewportHeight / cfg.visibleCount : 0),
        getTargetItem: () => targetWeekForAnchor(untrack(readAnchor), getView()),
        onSyncAnchor: (leadingIndex) => {
            if (getView() === DateGridView.Month) {
                const active = dominantMonthInViewport(
                    rawWeeks,
                    window.scrollPos,
                    window.viewportSize,
                    window.itemSize,
                    monthKeyOf(anchor),
                );
                if (!sameMonthKey(active, monthKeyOf(anchor))) {
                    anchor = new Date(active.year, active.month, 1);
                }
                return;
            }
            const visibleTop = rawWeeks[leadingIndex]?.[0];
            if (visibleTop && dayDiff(visibleTop, startOfWeek(anchor)) !== 0) {
                anchor = visibleTop;
            }
        },
    });

    const rawWeeks = $derived(buildInfiniteWeeks(window.windowStart, windowLength(window.config)));
    const range = $derived({
        startDate: ymd(rawWeeks[0][0]),
        endDate: ymd(rawWeeks[rawWeeks.length - 1][DAYS_IN_WEEK - 1]),
    });
    const dominantMonth = $derived<MonthKey>(
        dominantMonthInViewport(rawWeeks, window.scrollPos, window.viewportSize, window.itemSize, monthKeyOf(anchor)),
    );
    const topWeekStart = $derived.by(() => {
        if (window.itemSize <= 0) return targetWeekForAnchor(anchor, getView());
        return rawWeeks[window.leadingIndex]?.[0] ?? rawWeeks[0][0];
    });
    const weeks = $derived(
        tagOutOfMonthWeeks(rawWeeks, getView() === DateGridView.Month, dominantMonth),
    );

    function goToDate(target: Date, smooth = true): void {
        const normalized = startOfLocalDay(target);
        anchor = normalized;
        window.scrollToItem(targetWeekForAnchor(normalized, getView()), smooth);
    }

    return {
        get weekHeight() { return window.itemSize; },
        get range() { return range; },
        get dominantMonth() { return dominantMonth; },
        get topWeekStart() { return topWeekStart; },
        get weeks() { return weeks; },
        settle: window.settle,
        beginSnap: window.beginSnap,
        attach: window.attach,
        goToDate,
        shift(step: number): void {
            const view = getView();
            const base = view === DateGridView.Month
                ? new Date(anchor.getFullYear(), anchor.getMonth(), 1)
                : anchor;
            goToDate(shiftDateGridAnchor(base, view, step), true);
        },
    };
}
