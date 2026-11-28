import {
    attachAxisSnap,
    nearestBoundaryTarget,
    snapAxisScroll,
} from '../snapController';

export interface AttachWeekSnapOptions {
    /** Invoked once scrolling is idle right before measuring/snapping so infinite buffers can recenter first. */
    onBeforeSnap?: () => void;
    /** Invoked right before a smooth snap starts so callers avoid recentering mid-snap. */
    onSnapStart?: () => void;
}

/** Scroll offset aligning the week row nearest to `scrollTop` flush with the viewport top. */
export function weekSnapTarget(
    scrollTop: number,
    rowTops: readonly number[],
    maxScrollTop: number,
): number {
    return nearestBoundaryTarget(scrollTop, rowTops, maxScrollTop);
}

function measureRowTops(scroller: HTMLElement): number[] {
    const scrollerTop = scroller.getBoundingClientRect().top;
    const scrollTop = scroller.scrollTop;
    return Array.from(scroller.querySelectorAll<HTMLElement>('.cal-week'), (row) =>
        row.getBoundingClientRect().top - scrollerTop + scrollTop,
    );
}

/** Smoothly align the nearest week row to the top of `scroller`. */
export function snapWeekScroll(scroller: HTMLElement): boolean {
    return snapAxisScroll(scroller, 'top', measureRowTops(scroller));
}

/** Aligns vertical DateGrid scrolling smoothly onto week-row boundaries once scrolling settles. */
export function attachWeekSnap(
    scroller: HTMLElement,
    options: AttachWeekSnapOptions = {},
): () => void {
    return attachAxisSnap(scroller, {
        axis: 'top',
        measureBoundaries: measureRowTops,
        onBeforeSnap: options.onBeforeSnap,
        onSnapStart: options.onSnapStart,
    });
}
