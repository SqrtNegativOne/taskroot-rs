import { attachAxisSnap } from '../snapController';

export interface AttachWeekSnapOptions {
    /** Invoked once scrolling is idle right before measuring/snapping so infinite buffers can recenter first. */
    onBeforeSnap?: () => void;
    /** Invoked right before a smooth snap starts so callers avoid recentering mid-snap. */
    onSnapStart?: () => void;
}

function measureRowTops(scroller: HTMLElement): number[] {
    const scrollerTop = scroller.getBoundingClientRect().top;
    const scrollTop = scroller.scrollTop;
    return Array.from(scroller.querySelectorAll<HTMLElement>('.cal-week'), (row) =>
        row.getBoundingClientRect().top - scrollerTop + scrollTop,
    );
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
