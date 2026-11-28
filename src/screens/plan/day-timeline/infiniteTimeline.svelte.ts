import { flushSync, tick, untrack } from 'svelte';
import { addDays, dayDiff, ymd } from '../../../lib/time';
import {
    attachInfiniteScrollListeners,
    canScrollWithinWindow,
    leadingItemIndex,
    recenterShift,
    scrollElementTo,
    settleRecenterShift,
    startOfLocalDay,
    windowLength,
    type InfiniteWindowConfig,
} from '../infiniteScroll';

const DEFAULT_GUTTER_WIDTH_PX = 44;
const MIN_COLUMN_WIDTH_PX = 40;
const EMERGENCY_MARGIN_DAYS = 2;
const SETTLE_DRIFT_DAYS = 3;
const ALREADY_ALIGNED_EPSILON_PX = 1;

export function timelineWindowConfig(numDays: number): InfiniteWindowConfig {
    const visibleCount = Math.max(1, numDays);
    const bufferCount = Math.max(14, visibleCount * 2 + 6);
    return { visibleCount, bufferCount, recenterMargin: EMERGENCY_MARGIN_DAYS };
}

export function timelineColumnWidth(
    viewportWidth: number,
    gutterWidth: number,
    numDays: number,
): number {
    if (viewportWidth <= gutterWidth || numDays <= 0) return 0;
    return Math.max(MIN_COLUMN_WIDTH_PX, (viewportWidth - gutterWidth) / numDays);
}

function measureGutterWidth(scroller: HTMLElement): number {
    const gutter = scroller.querySelector<HTMLElement>('.day-gutter');
    const measured = gutter?.getBoundingClientRect().width ?? 0;
    return measured > 0 ? measured : DEFAULT_GUTTER_WIDTH_PX;
}

function dayColumnElement(scroller: HTMLElement, date: Date): HTMLElement | null {
    return scroller.querySelector<HTMLElement>(`.day-grid[data-drop-date="${ymd(date)}"]`);
}

function dayColumnScrollLeft(
    scroller: HTMLElement,
    date: Date,
    fallbackLeft: number,
): number {
    const first = scroller.querySelector<HTMLElement>('.day-grid');
    const target = dayColumnElement(scroller, date);
    if (!first || !target) return fallbackLeft;
    return Math.max(0, target.getBoundingClientRect().left - first.getBoundingClientRect().left);
}

class InfiniteTimelineController {
    viewDate = $state(startOfLocalDay(new Date()));
    private windowStart = $state(startOfLocalDay(new Date()));
    private viewportWidth = $state(0);
    private gutterWidth = $state(DEFAULT_GUTTER_WIDTH_PX);
    private scroller = $state<HTMLElement | null>(null);
    private programmaticScroll = false;
    private alignToken = 0;

    config = $derived.by(() => timelineWindowConfig(this.getNumDays()));
    colWidth = $derived(timelineColumnWidth(this.viewportWidth, this.gutterWidth, this.config.visibleCount));
    dates = $derived(
        Array.from({ length: windowLength(this.config) }, (_, i) => addDays(this.windowStart, i)),
    );
    dateRange = $derived({
        startDate: ymd(this.dates[0]),
        endDate: ymd(this.dates[this.dates.length - 1]),
    });

    constructor(
        private getNumDays: () => number,
        initialDate: Date,
    ) {
        const initialDay = startOfLocalDay(initialDate);
        const initialConfig = timelineWindowConfig(untrack(getNumDays));
        this.viewDate = initialDay;
        this.windowStart = addDays(initialDay, -initialConfig.bufferCount);

        $effect(() => {
            this.alignAfterGeometryChange();
        });
    }

    private alignAfterGeometryChange(): void {
        const el = this.scroller;
        const width = this.colWidth;
        const cfg = this.config;
        if (!el || width <= 0) return;

        const target = untrack(() => this.viewDate);
        const start = untrack(() => this.windowStart);
        const index = dayDiff(target, start);
        const maxSafe = cfg.bufferCount * 2 - cfg.recenterMargin;

        this.programmaticScroll = false;
        if (index < cfg.recenterMargin || index > maxSafe) {
            this.windowStart = addDays(target, -cfg.bufferCount);
            const token = ++this.alignToken;
            void tick().then(() => {
                if (token !== this.alignToken || this.scroller !== el) return;
                const left = dayColumnScrollLeft(el, target, cfg.bufferCount * width);
                scrollElementTo(el, 'left', left, false);
            });
            return;
        }
        const left = dayColumnScrollLeft(el, target, index * width);
        scrollElementTo(el, 'left', left, false);
    }

    private measure(el: HTMLElement): void {
        this.viewportWidth = el.clientWidth;
        this.gutterWidth = measureGutterWidth(el);
    }

    private applyShift(el: HTMLElement, shift: number, cancelSmooth: boolean): void {
        if (shift === 0 || this.colWidth <= 0) return;
        const leadingIdx = leadingItemIndex(el.scrollLeft, this.colWidth, this.config.bufferCount * 2);
        const survivingDay = this.dates[leadingIdx];
        const survivingEl = survivingDay ? dayColumnElement(el, survivingDay) : null;
        const beforeLeft = survivingEl?.getBoundingClientRect().left;

        flushSync(() => {
            this.windowStart = addDays(this.windowStart, shift);
        });

        const afterLeft = survivingEl?.getBoundingClientRect().left;
        const delta =
            beforeLeft !== undefined && afterLeft !== undefined && survivingEl?.isConnected
                ? afterLeft - beforeLeft
                : -shift * this.colWidth;
        const nextLeft = el.scrollLeft + delta;
        if (cancelSmooth) {
            scrollElementTo(el, 'left', nextLeft, false);
        } else {
            el.scrollLeft = nextLeft;
        }
    }

    private syncViewDateFromScroll(el: HTMLElement): void {
        if (this.colWidth <= 0) return;
        const idx = leadingItemIndex(el.scrollLeft, this.colWidth, this.config.bufferCount * 2);
        const visible = addDays(this.windowStart, idx);
        if (dayDiff(visible, this.viewDate) !== 0) {
            this.viewDate = visible;
        }
    }

    private handleScroll(el: HTMLElement): void {
        if (el.clientWidth !== this.viewportWidth) {
            this.measure(el);
            return;
        }
        if (this.colWidth <= 0 || this.programmaticScroll) return;

        const emergency = recenterShift(el.scrollLeft, this.colWidth, this.config);
        this.applyShift(el, emergency, false);
        this.syncViewDateFromScroll(el);
    }

    private handleScrollEnd(el: HTMLElement): void {
        const wasProgrammatic = this.programmaticScroll;
        this.programmaticScroll = false;
        if (this.colWidth <= 0) return;
        if (!wasProgrammatic) {
            this.syncViewDateFromScroll(el);
        }
    }

    /** Recenters once scrolling has genuinely settled, right before day snapping measures columns. */
    settle = (): void => {
        const el = this.scroller;
        if (!el || this.colWidth <= 0) return;
        const shift = settleRecenterShift(el.scrollLeft, this.colWidth, this.config, SETTLE_DRIFT_DAYS);
        this.applyShift(el, shift, true);
        this.syncViewDateFromScroll(el);
    };

    /** Suppresses emergency recentering while `attachDaySnap` runs its smooth glide. */
    beginSnap = (): void => {
        this.programmaticScroll = true;
    };

    goToDate = (target: Date, smooth = true): void => {
        const normalized = startOfLocalDay(target);
        this.viewDate = normalized;
        const el = this.scroller;
        if (!el || this.colWidth <= 0) {
            this.windowStart = addDays(normalized, -this.config.bufferCount);
            return;
        }

        const targetIndex = dayDiff(normalized, this.windowStart);
        if (canScrollWithinWindow(targetIndex, this.config)) {
            const dest = dayColumnScrollLeft(el, normalized, targetIndex * this.colWidth);
            if (Math.abs(dest - el.scrollLeft) < ALREADY_ALIGNED_EPSILON_PX) {
                this.programmaticScroll = false;
                return;
            }
            this.programmaticScroll = smooth;
            scrollElementTo(el, 'left', dest, smooth);
            if (!smooth) {
                const shift = settleRecenterShift(el.scrollLeft, this.colWidth, this.config, SETTLE_DRIFT_DAYS);
                this.applyShift(el, shift, true);
            }
            return;
        }

        this.programmaticScroll = false;
        flushSync(() => {
            this.windowStart = addDays(normalized, -this.config.bufferCount);
        });
        const dest = dayColumnScrollLeft(el, normalized, this.config.bufferCount * this.colWidth);
        scrollElementTo(el, 'left', dest, false);
    };

    attach = (el: HTMLElement): (() => void) => {
        this.scroller = el;
        this.measure(el);
        const detach = attachInfiniteScrollListeners(el, {
            onScroll: () => { this.handleScroll(el); },
            onScrollEnd: () => { this.handleScrollEnd(el); },
            onUserInput: () => { this.programmaticScroll = false; },
            onResize: () => { this.measure(el); },
        });
        return () => {
            detach();
            if (this.scroller === el) this.scroller = null;
        };
    };
}

/**
 * Horizontal infinite-scroll controller for `DayTimeline`. Keeps a buffered
 * sliding window of day columns centered around `viewDate` and derives each
 * column's width from the available viewport divided by `numDays`.
 */
export function createInfiniteTimeline(
    getNumDays: () => number,
    initialDate: Date = new Date(),
): InfiniteTimelineController {
    return new InfiniteTimelineController(getNumDays, initialDate);
}
