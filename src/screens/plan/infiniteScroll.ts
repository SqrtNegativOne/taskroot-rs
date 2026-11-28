export interface InfiniteWindowConfig {
    visibleCount: number;
    bufferCount: number;
    recenterMargin: number;
}

export function windowLength(config: InfiniteWindowConfig): number {
    return config.bufferCount * 2 + config.visibleCount;
}

export function leadingItemIndex(
    scrollPos: number,
    itemSize: number,
    maxIndex: number,
): number {
    if (itemSize <= 0 || maxIndex <= 0) return 0;
    return Math.min(maxIndex, Math.max(0, Math.round(scrollPos / itemSize)));
}

/**
 * Number of items to shift `windowStart` by when `scrollPos` crosses outside
 * `[margin, bufferCount * 2 - margin]`. Returns `0` while safely inside.
 */
export function recenterShift(
    scrollPos: number,
    itemSize: number,
    config: InfiniteWindowConfig,
    margin: number = config.recenterMargin,
): number {
    if (itemSize <= 0) return 0;
    const offset = scrollPos / itemSize;
    const maxSafe = config.bufferCount * 2 - margin;
    if (offset >= margin && offset <= maxSafe) return 0;
    return Math.round(offset) - config.bufferCount;
}

/**
 * Settle-time recenter shift used on `scrollend` once scrolling has stopped:
 * brings the window back to center whenever the user drifted at least
 * `threshold` items away, without ever firing mid-gesture.
 */
export function settleRecenterShift(
    scrollPos: number,
    itemSize: number,
    config: InfiniteWindowConfig,
    threshold = 2,
): number {
    if (itemSize <= 0) return 0;
    const drift = Math.round(scrollPos / itemSize) - config.bufferCount;
    return Math.abs(drift) >= threshold ? drift : 0;
}

/** Whether `targetIndex` lies within the currently rendered scrollable window. */
export function canScrollWithinWindow(
    targetIndex: number,
    config: InfiniteWindowConfig,
): boolean {
    return targetIndex >= 0 && targetIndex <= config.bufferCount * 2;
}

export type ScrollAxis = 'left' | 'top';

export function startOfLocalDay(date: Date): Date {
    return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

export function readScrollPos(el: HTMLElement, axis: ScrollAxis): number {
    return axis === 'left' ? el.scrollLeft : el.scrollTop;
}

export function writeScrollPos(el: HTMLElement, axis: ScrollAxis, position: number): void {
    if (axis === 'left') {
        el.scrollLeft = position;
    } else {
        el.scrollTop = position;
    }
}

export function readViewportSize(el: HTMLElement, axis: ScrollAxis): number {
    return axis === 'left' ? el.clientWidth : el.clientHeight;
}

export function readEdge(el: HTMLElement, axis: ScrollAxis): number {
    const rect = el.getBoundingClientRect();
    return axis === 'left' ? rect.left : rect.top;
}

export function measureItemOffset(
    scroller: HTMLElement,
    axis: ScrollAxis,
    firstSelector: string,
    target: HTMLElement | null,
    fallback: number,
): number {
    const first = scroller.querySelector<HTMLElement>(firstSelector);
    if (!first || !target) return fallback;
    return Math.max(0, readEdge(target, axis) - readEdge(first, axis));
}

export function scrollElementTo(
    element: HTMLElement,
    axis: ScrollAxis,
    position: number,
    smooth: boolean,
): void {
    if (typeof element.scrollTo === 'function') {
        // Explicit 'instant' cancels any in-flight smooth scroll in Chromium.
        element.scrollTo({ [axis]: position, behavior: smooth ? 'smooth' : 'instant' });
        return;
    }
    writeScrollPos(element, axis, position);
}

export interface ScrollObserverHandlers {
    onScroll: () => void;
    onScrollEnd: () => void;
    onUserInput: () => void;
    onResize: () => void;
}

/** Attaches scroll, scrollend, user-input cancellation and ResizeObserver with teardown. */
export function attachInfiniteScrollListeners(
    el: HTMLElement,
    handlers: ScrollObserverHandlers,
): () => void {
    el.addEventListener('scroll', handlers.onScroll, { passive: true });
    el.addEventListener('scrollend', handlers.onScrollEnd);
    el.addEventListener('wheel', handlers.onUserInput, { passive: true });
    el.addEventListener('pointerdown', handlers.onUserInput, { passive: true });
    el.addEventListener('touchstart', handlers.onUserInput, { passive: true });

    let observer: ResizeObserver | undefined;
    if (typeof ResizeObserver !== 'undefined') {
        observer = new ResizeObserver(handlers.onResize);
        observer.observe(el);
    }

    return () => {
        observer?.disconnect();
        el.removeEventListener('scroll', handlers.onScroll);
        el.removeEventListener('scrollend', handlers.onScrollEnd);
        el.removeEventListener('wheel', handlers.onUserInput);
        el.removeEventListener('pointerdown', handlers.onUserInput);
        el.removeEventListener('touchstart', handlers.onUserInput);
    };
}
