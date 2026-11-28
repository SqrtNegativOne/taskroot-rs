import { flushSync, tick, untrack } from 'svelte';
import { addDays, dayDiff, startOfWeek, ymd } from '../../../lib/time';
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

const EMERGENCY_MARGIN_WEEKS = 2;
const SETTLE_DRIFT_WEEKS = 2;
const ALREADY_ALIGNED_EPSILON_PX = 1;

function gridWindowConfig(view: DateGridView): InfiniteWindowConfig {
    const visibleCount = VISIBLE_WEEKS[view];
    const bufferCount = Math.max(8, visibleCount + 4);
    return { visibleCount, bufferCount, recenterMargin: EMERGENCY_MARGIN_WEEKS };
}

function weekOffsetBetween(targetWeek: Date, windowStartWeek: Date): number {
    return Math.round(dayDiff(targetWeek, windowStartWeek) / DAYS_IN_WEEK);
}

function weekRowElement(scroller: HTMLElement, weekStart: Date): HTMLElement | null {
    return scroller.querySelector<HTMLElement>(`[data-week-start="${ymd(weekStart)}"]`);
}

function weekRowTop(scroller: HTMLElement, weekStart: Date, fallbackTop: number): number {
    const first = scroller.querySelector<HTMLElement>('.cal-week');
    const target = weekRowElement(scroller, weekStart);
    if (!first || !target) return fallbackTop;
    return Math.max(0, target.getBoundingClientRect().top - first.getBoundingClientRect().top);
}

class InfiniteDateGridController {
    private anchor = $state(startOfLocalDay(new Date()));
    private windowStartWeek = $state(startOfWeek(startOfLocalDay(new Date())));
    private viewportHeight = $state(0);
    private scrollTop = $state(0);
    private scroller = $state<HTMLElement | null>(null);
    private suppressRecenter = false;
    private lockAnchor = false;
    private alignToken = 0;

    private config = $derived.by(() => gridWindowConfig(this.getView()));
    weekHeight = $derived(this.viewportHeight > 0 ? this.viewportHeight / this.config.visibleCount : 0);
    private rawWeeks = $derived(buildInfiniteWeeks(this.windowStartWeek, windowLength(this.config)));
    range = $derived({
        startDate: ymd(this.rawWeeks[0][0]),
        endDate: ymd(this.rawWeeks[this.rawWeeks.length - 1][DAYS_IN_WEEK - 1]),
    });
    dominantMonth = $derived<MonthKey>(
        dominantMonthInViewport(
            this.rawWeeks,
            this.scrollTop,
            this.viewportHeight,
            this.weekHeight,
            monthKeyOf(this.anchor),
        ),
    );
    topWeekStart = $derived.by(() => {
        if (this.weekHeight <= 0) return targetWeekForAnchor(this.anchor, this.getView());
        const idx = leadingItemIndex(this.scrollTop, this.weekHeight, this.config.bufferCount * 2);
        return this.rawWeeks[idx]?.[0] ?? this.rawWeeks[0][0];
    });
    weeks = $derived.by(() =>
        tagOutOfMonthWeeks(this.rawWeeks, this.getView() === DateGridView.Month, this.dominantMonth),
    );

    constructor(
        private getView: () => DateGridView,
        initialDate: Date,
    ) {
        const initialDay = startOfLocalDay(initialDate);
        const initialView = untrack(getView);
        const initialConfig = gridWindowConfig(initialView);
        this.anchor = initialDay;
        this.windowStartWeek = addDays(
            targetWeekForAnchor(initialDay, initialView),
            -initialConfig.bufferCount * DAYS_IN_WEEK,
        );

        $effect(() => {
            this.alignAfterGeometryChange();
        });
    }

    private alignAfterGeometryChange(): void {
        const el = this.scroller;
        const rowH = this.weekHeight;
        const view = this.getView();
        const cfg = this.config;
        if (!el || rowH <= 0) return;

        const targetWeek = targetWeekForAnchor(untrack(() => this.anchor), view);
        const start = untrack(() => this.windowStartWeek);
        const index = weekOffsetBetween(targetWeek, start);
        const maxSafe = cfg.bufferCount * 2 - cfg.recenterMargin;

        this.suppressRecenter = false;
        this.lockAnchor = false;
        if (index < cfg.recenterMargin || index > maxSafe) {
            this.windowStartWeek = addDays(targetWeek, -cfg.bufferCount * DAYS_IN_WEEK);
            const token = ++this.alignToken;
            void tick().then(() => {
                if (token !== this.alignToken || this.scroller !== el) return;
                const top = weekRowTop(el, targetWeek, cfg.bufferCount * rowH);
                scrollElementTo(el, 'top', top, false);
                this.scrollTop = el.scrollTop;
            });
            return;
        }
        const top = weekRowTop(el, targetWeek, index * rowH);
        scrollElementTo(el, 'top', top, false);
        this.scrollTop = el.scrollTop;
    }

    private applyShift(el: HTMLElement, shift: number, cancelSmooth: boolean): void {
        if (shift === 0 || this.weekHeight <= 0) return;
        const leadingIdx = leadingItemIndex(el.scrollTop, this.weekHeight, this.config.bufferCount * 2);
        const survivingWeek = this.rawWeeks[leadingIdx]?.[0];
        const survivingEl = survivingWeek ? weekRowElement(el, survivingWeek) : null;
        const beforeTop = survivingEl?.getBoundingClientRect().top;

        flushSync(() => {
            this.windowStartWeek = addDays(this.windowStartWeek, shift * DAYS_IN_WEEK);
        });

        const afterTop = survivingEl?.getBoundingClientRect().top;
        const delta =
            beforeTop !== undefined && afterTop !== undefined && survivingEl?.isConnected
                ? afterTop - beforeTop
                : -shift * this.weekHeight;
        const nextTop = el.scrollTop + delta;
        if (cancelSmooth) {
            scrollElementTo(el, 'top', nextTop, false);
        } else {
            el.scrollTop = nextTop;
        }
        this.scrollTop = el.scrollTop;
    }

    private syncAnchorFromScroll(): void {
        if (this.weekHeight <= 0) return;
        if (this.getView() === DateGridView.Month) {
            const active = dominantMonthInViewport(
                this.rawWeeks,
                this.scrollTop,
                this.viewportHeight,
                this.weekHeight,
                monthKeyOf(this.anchor),
            );
            if (!sameMonthKey(active, monthKeyOf(this.anchor))) {
                this.anchor = new Date(active.year, active.month, 1);
            }
            return;
        }
        const idx = leadingItemIndex(this.scrollTop, this.weekHeight, this.config.bufferCount * 2);
        const visibleTop = this.rawWeeks[idx]?.[0];
        if (visibleTop && dayDiff(visibleTop, startOfWeek(this.anchor)) !== 0) {
            this.anchor = visibleTop;
        }
    }

    private handleScroll(el: HTMLElement): void {
        if (el.clientHeight !== this.viewportHeight) {
            this.viewportHeight = el.clientHeight;
            return;
        }
        this.scrollTop = el.scrollTop;
        if (this.weekHeight <= 0 || this.suppressRecenter) return;

        const emergency = recenterShift(el.scrollTop, this.weekHeight, this.config);
        this.applyShift(el, emergency, false);
        this.syncAnchorFromScroll();
    }

    private handleScrollEnd(el: HTMLElement): void {
        const keepAnchor = this.lockAnchor;
        this.suppressRecenter = false;
        this.lockAnchor = false;
        this.scrollTop = el.scrollTop;
        if (this.weekHeight <= 0) return;
        if (!keepAnchor) {
            this.syncAnchorFromScroll();
        }
    }

    /** Recenters once scrolling has genuinely settled, right before week snapping measures rows. */
    settle = (): void => {
        const el = this.scroller;
        if (!el || this.weekHeight <= 0) return;
        this.scrollTop = el.scrollTop;
        const shift = settleRecenterShift(el.scrollTop, this.weekHeight, this.config, SETTLE_DRIFT_WEEKS);
        this.applyShift(el, shift, true);
        if (!this.lockAnchor) {
            this.syncAnchorFromScroll();
        }
    };

    /** Suppresses emergency recentering while `attachWeekSnap` runs its smooth glide. */
    beginSnap = (): void => {
        this.suppressRecenter = true;
    };

    goToDate = (target: Date, smooth = true): void => {
        const normalized = startOfLocalDay(target);
        const view = this.getView();
        this.anchor = normalized;
        const targetWeek = targetWeekForAnchor(normalized, view);
        const el = this.scroller;
        if (!el || this.weekHeight <= 0) {
            this.windowStartWeek = addDays(targetWeek, -this.config.bufferCount * DAYS_IN_WEEK);
            return;
        }

        const index = weekOffsetBetween(targetWeek, this.windowStartWeek);
        if (canScrollWithinWindow(index, this.config)) {
            const dest = weekRowTop(el, targetWeek, index * this.weekHeight);
            if (Math.abs(dest - el.scrollTop) < ALREADY_ALIGNED_EPSILON_PX) {
                this.suppressRecenter = false;
                this.lockAnchor = false;
                return;
            }
            this.suppressRecenter = smooth;
            this.lockAnchor = smooth;
            scrollElementTo(el, 'top', dest, smooth);
            if (!smooth) {
                this.scrollTop = el.scrollTop;
                const shift = settleRecenterShift(el.scrollTop, this.weekHeight, this.config, SETTLE_DRIFT_WEEKS);
                this.applyShift(el, shift, true);
            }
            return;
        }

        this.suppressRecenter = false;
        this.lockAnchor = false;
        flushSync(() => {
            this.windowStartWeek = addDays(targetWeek, -this.config.bufferCount * DAYS_IN_WEEK);
        });
        const dest = weekRowTop(el, targetWeek, this.config.bufferCount * this.weekHeight);
        scrollElementTo(el, 'top', dest, false);
        this.scrollTop = el.scrollTop;
    };

    shift = (step: number): void => {
        const view = this.getView();
        const base =
            view === DateGridView.Month
                ? new Date(this.anchor.getFullYear(), this.anchor.getMonth(), 1)
                : this.anchor;
        this.goToDate(shiftDateGridAnchor(base, view, step), true);
    };

    attach = (el: HTMLElement): (() => void) => {
        this.scroller = el;
        this.viewportHeight = el.clientHeight;
        const detach = attachInfiniteScrollListeners(el, {
            onScroll: () => { this.handleScroll(el); },
            onScrollEnd: () => { this.handleScrollEnd(el); },
            onUserInput: () => {
                this.suppressRecenter = false;
                this.lockAnchor = false;
            },
            onResize: () => { this.viewportHeight = el.clientHeight; },
        });
        return () => {
            detach();
            if (this.scroller === el) this.scroller = null;
        };
    };
}

/**
 * Vertical infinite-scroll controller for `DateGrid`. Keeps a buffered sliding
 * window of week rows, sizes each week row from the viewport height divided by
 * the view's visible week count, and tracks the most-visible calendar month.
 */
export function createInfiniteDateGrid(
    getView: () => DateGridView,
    initialDate: Date = new Date(),
): InfiniteDateGridController {
    return new InfiniteDateGridController(getView, initialDate);
}
