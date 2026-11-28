import { flushSync, tick, untrack } from 'svelte';
import { addDays, dayDiff } from '../../lib/time';
import {
    attachInfiniteScrollListeners,
    canScrollWithinWindow,
    leadingItemIndex,
    measureItemOffset,
    readEdge,
    readScrollPos,
    readViewportSize,
    recenterShift,
    scrollElementTo,
    settleRecenterShift,
    writeScrollPos,
    type InfiniteWindowConfig,
    type ScrollAxis,
} from './infiniteScroll';

const ALREADY_ALIGNED_EPSILON_PX = 1;

export interface InfiniteWindowOptions {
    axis: ScrollAxis;
    stepDays: number;
    settleDrift: number;
    initialTargetItem: Date;
    firstItemSelector: string;
    findItemElement: (scroller: HTMLElement, itemStart: Date) => HTMLElement | null;
    getConfig: () => InfiniteWindowConfig;
    getItemSize: (viewportSize: number, config: InfiniteWindowConfig) => number;
    getTargetItem: () => Date;
    onSyncAnchor: (leadingIndex: number) => void;
    onMeasure?: (scroller: HTMLElement) => void;
}

export class InfiniteWindowController {
    windowStart = $state(new Date());
    viewportSize = $state(0);
    scrollPos = $state(0);
    private scroller = $state<HTMLElement | null>(null);
    private suppressRecenter = false;
    private lockAnchor = false;
    private alignToken = 0;

    config = $derived.by(() => this.options.getConfig());
    itemSize = $derived.by(() => this.options.getItemSize(this.viewportSize, this.config));
    leadingIndex = $derived(leadingItemIndex(this.scrollPos, this.itemSize, this.config.bufferCount * 2));

    constructor(private options: InfiniteWindowOptions) {
        const initialConfig = untrack(options.getConfig);
        this.windowStart = this.shiftItem(options.initialTargetItem, -initialConfig.bufferCount);
        $effect(() => { this.alignAfterGeometryChange(); });
    }

    private shiftItem(base: Date, count: number): Date {
        return addDays(base, count * this.options.stepDays);
    }

    private itemOffset(target: Date, start: Date): number {
        const days = dayDiff(target, start);
        return this.options.stepDays === 1 ? days : Math.round(days / this.options.stepDays);
    }

    private measure(el: HTMLElement): void {
        this.viewportSize = readViewportSize(el, this.options.axis);
        this.options.onMeasure?.(el);
    }

    private itemOffsetPx(el: HTMLElement, itemStart: Date, fallback: number): number {
        return measureItemOffset(
            el,
            this.options.axis,
            this.options.firstItemSelector,
            this.options.findItemElement(el, itemStart),
            fallback,
        );
    }

    private alignAfterGeometryChange(): void {
        const el = this.scroller;
        const size = this.itemSize;
        const cfg = this.config;
        const target = this.options.getTargetItem();
        if (!el || size <= 0) return;

        const index = this.itemOffset(target, untrack(() => this.windowStart));
        const maxSafe = cfg.bufferCount * 2 - cfg.recenterMargin;

        this.suppressRecenter = false;
        this.lockAnchor = false;
        if (index < cfg.recenterMargin || index > maxSafe) {
            this.windowStart = this.shiftItem(target, -cfg.bufferCount);
            const token = ++this.alignToken;
            void tick().then(() => {
                if (token !== this.alignToken || this.scroller !== el) return;
                scrollElementTo(el, this.options.axis, this.itemOffsetPx(el, target, cfg.bufferCount * size), false);
                this.scrollPos = readScrollPos(el, this.options.axis);
            });
            return;
        }
        scrollElementTo(el, this.options.axis, this.itemOffsetPx(el, target, index * size), false);
        this.scrollPos = readScrollPos(el, this.options.axis);
    }

    private applyShift(el: HTMLElement, shift: number, cancelSmooth: boolean): void {
        if (shift === 0 || this.itemSize <= 0) return;
        const axis = this.options.axis;
        const leadingIdx = leadingItemIndex(readScrollPos(el, axis), this.itemSize, this.config.bufferCount * 2);
        const survivingEl = this.options.findItemElement(el, this.shiftItem(this.windowStart, leadingIdx));
        const beforeEdge = survivingEl ? readEdge(survivingEl, axis) : undefined;

        flushSync(() => { this.windowStart = this.shiftItem(this.windowStart, shift); });

        const afterEdge = survivingEl ? readEdge(survivingEl, axis) : undefined;
        const delta =
            beforeEdge !== undefined && afterEdge !== undefined && survivingEl?.isConnected
                ? afterEdge - beforeEdge
                : -shift * this.itemSize;
        const nextPos = readScrollPos(el, axis) + delta;
        if (cancelSmooth) {
            scrollElementTo(el, axis, nextPos, false);
        } else {
            writeScrollPos(el, axis, nextPos);
        }
        this.scrollPos = readScrollPos(el, axis);
    }

    private syncAnchor(): void {
        if (this.itemSize <= 0) return;
        this.options.onSyncAnchor(leadingItemIndex(this.scrollPos, this.itemSize, this.config.bufferCount * 2));
    }

    private handleScroll(el: HTMLElement): void {
        if (readViewportSize(el, this.options.axis) !== this.viewportSize) {
            this.measure(el);
            return;
        }
        this.scrollPos = readScrollPos(el, this.options.axis);
        if (this.itemSize <= 0 || this.suppressRecenter) return;

        this.applyShift(el, recenterShift(this.scrollPos, this.itemSize, this.config), false);
        this.syncAnchor();
    }

    private handleScrollEnd(el: HTMLElement): void {
        const keepAnchor = this.lockAnchor;
        this.suppressRecenter = false;
        this.lockAnchor = false;
        this.scrollPos = readScrollPos(el, this.options.axis);
        if (this.itemSize > 0 && !keepAnchor) {
            this.syncAnchor();
        }
    }

    private settleRecenter(el: HTMLElement): void {
        this.scrollPos = readScrollPos(el, this.options.axis);
        const shift = settleRecenterShift(this.scrollPos, this.itemSize, this.config, this.options.settleDrift);
        this.applyShift(el, shift, true);
    }

    /** Recenters once scrolling has genuinely settled, right before snap alignment measures items. */
    settle = (): void => {
        const el = this.scroller;
        if (!el || this.itemSize <= 0) return;
        this.settleRecenter(el);
        if (!this.lockAnchor) {
            this.syncAnchor();
        }
    };

    /** Suppresses emergency recentering while a snap controller runs its smooth glide. */
    beginSnap = (): void => {
        this.suppressRecenter = true;
    };

    /** Scrolls smoothly (or jumps) to place `targetItem` at the viewport origin. */
    scrollToItem = (targetItem: Date, smooth = true): void => {
        const el = this.scroller;
        const size = this.itemSize;
        const cfg = this.config;
        const axis = this.options.axis;
        if (!el || size <= 0) {
            this.windowStart = this.shiftItem(targetItem, -cfg.bufferCount);
            return;
        }

        const index = this.itemOffset(targetItem, this.windowStart);
        if (canScrollWithinWindow(index, cfg)) {
            const dest = this.itemOffsetPx(el, targetItem, index * size);
            if (Math.abs(dest - readScrollPos(el, axis)) < ALREADY_ALIGNED_EPSILON_PX) {
                this.suppressRecenter = false;
                this.lockAnchor = false;
                return;
            }
            this.suppressRecenter = smooth;
            this.lockAnchor = smooth;
            scrollElementTo(el, axis, dest, smooth);
            if (!smooth) this.settleRecenter(el);
            return;
        }

        this.suppressRecenter = false;
        this.lockAnchor = false;
        flushSync(() => { this.windowStart = this.shiftItem(targetItem, -cfg.bufferCount); });
        scrollElementTo(el, axis, this.itemOffsetPx(el, targetItem, cfg.bufferCount * size), false);
        this.scrollPos = readScrollPos(el, axis);
    };

    attach = (el: HTMLElement): (() => void) => {
        this.scroller = el;
        this.measure(el);
        const detach = attachInfiniteScrollListeners(el, {
            onScroll: () => { this.handleScroll(el); },
            onScrollEnd: () => { this.handleScrollEnd(el); },
            onUserInput: () => {
                this.suppressRecenter = false;
                this.lockAnchor = false;
            },
            onResize: () => { this.measure(el); },
        });
        return () => {
            detach();
            if (this.scroller === el) this.scroller = null;
        };
    };
}

/**
 * Axis-parameterized infinite-scroll window controller. Maintains a buffered
 * sliding window of date-stepped items, aligns scroll position across viewport
 * or view-mode geometry changes, and recenters seamlessly via `flushSync`.
 */
export function createInfiniteWindowController(options: InfiniteWindowOptions): InfiniteWindowController {
    return new InfiniteWindowController(options);
}
