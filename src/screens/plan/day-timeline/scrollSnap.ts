import {
    attachAxisSnap,
    nearestBoundaryTarget,
    snapAxisScroll,
} from '../snapController';

export interface AttachDaySnapOptions {
    /** Invoked once scrolling is idle right before measuring/snapping so infinite buffers can recenter first. */
    onBeforeSnap?: () => void;
    /** Invoked right before a smooth snap scroll starts so callers can avoid recentering mid-snap. */
    onSnapStart?: () => void;
}

/**
 * Scroll offset that puts the day column nearest to `scrollLeft` on the
 * timeline's start padding. Column lefts are scroll-independent content
 * coordinates measured against the first column, so the first day always maps
 * to offset 0 and unreachable columns clamp to the end of the scroll range.
 */
export function daySnapTarget(
    scrollLeft: number,
    columnLefts: readonly number[],
    maxScrollLeft: number,
): number {
    return nearestBoundaryTarget(scrollLeft, columnLefts, maxScrollLeft);
}

function measureColumnLefts(scroller: HTMLElement): number[] {
    const scrollerLeft = scroller.getBoundingClientRect().left;
    const scrollLeft = scroller.scrollLeft;
    return Array.from(scroller.querySelectorAll<HTMLElement>('.day-grid'), (column) =>
        column.getBoundingClientRect().left - scrollerLeft + scrollLeft,
    );
}

/** Smoothly align the nearest day column, or do nothing when already aligned. */
export function snapDayScroll(scroller: HTMLElement): boolean {
    return snapAxisScroll(scroller, 'left', measureColumnLefts(scroller));
}

/** Aligns horizontal scroll to day boundaries once scrolling genuinely settles. */
export function attachDaySnap(
    scroller: HTMLElement,
    options: AttachDaySnapOptions = {},
): () => void {
    return attachAxisSnap(scroller, {
        axis: 'left',
        measureBoundaries: measureColumnLefts,
        onBeforeSnap: options.onBeforeSnap,
        onSnapStart: options.onSnapStart,
    });
}
