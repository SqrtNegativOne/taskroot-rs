const IDLE_SNAP_DELAY_MS = 200;
const SNAP_EPSILON_PX = 1;

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
    const max = Math.max(0, maxScrollLeft);
    if (columnLefts.length === 0) return Math.min(Math.max(scrollLeft, 0), max);

    const origin = columnLefts[0];
    let best = 0;
    let bestDistance = Number.POSITIVE_INFINITY;
    for (const left of columnLefts) {
        const target = Math.min(Math.max(left - origin, 0), max);
        const distance = Math.abs(target - scrollLeft);
        if (distance < bestDistance) {
            best = target;
            bestDistance = distance;
        }
    }
    return best;
}

function measureColumnLefts(scroller: HTMLElement): number[] {
    const scrollerLeft = scroller.getBoundingClientRect().left;
    const scrollLeft = scroller.scrollLeft;
    return Array.from(scroller.querySelectorAll<HTMLElement>('.day-grid'), (column) =>
        column.getBoundingClientRect().left - scrollerLeft + scrollLeft,
    );
}

/** Align the nearest day column, or do nothing when the offset already sits on one. */
export function snapDayScroll(scroller: HTMLElement): void {
    const maxScrollLeft = scroller.scrollWidth - scroller.clientWidth;
    const target = daySnapTarget(scroller.scrollLeft, measureColumnLefts(scroller), maxScrollLeft);
    if (Math.abs(target - scroller.scrollLeft) < SNAP_EPSILON_PX) return;
    scroller.scrollLeft = target;
}

/** Whether the engine reports `scrollend`; kept in a function so TypeScript's `in`-narrowing does not eat `scroller`. */
function supportsScrollEnd(el: HTMLElement): boolean {
    return 'onscrollend' in el;
}

/**
 * Aligns the horizontal scroll to day boundaries: immediately when the browser
 * reports the end of a scroll gesture, otherwise once scrolling has been idle
 * briefly. Returns the teardown.
 */
export function attachDaySnap(scroller: HTMLElement): () => void {
    if (supportsScrollEnd(scroller)) {
        const onScrollEnd = (): void => snapDayScroll(scroller);
        scroller.addEventListener('scrollend', onScrollEnd);
        return () => scroller.removeEventListener('scrollend', onScrollEnd);
    }

    let idleTimer: ReturnType<typeof setTimeout> | undefined;
    const onScroll = (): void => {
        if (idleTimer !== undefined) clearTimeout(idleTimer);
        idleTimer = setTimeout(() => {
            idleTimer = undefined;
            snapDayScroll(scroller);
        }, IDLE_SNAP_DELAY_MS);
    };
    scroller.addEventListener('scroll', onScroll);
    return () => {
        if (idleTimer !== undefined) clearTimeout(idleTimer);
        scroller.removeEventListener('scroll', onScroll);
    };
}
