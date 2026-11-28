const IDLE_SNAP_DELAY_MS = 200;
const WHEEL_SETTLE_MS = 140;
const SNAP_EPSILON_PX = 1;

export type SnapAxis = 'left' | 'top';

export interface AttachAxisSnapOptions {
    axis: SnapAxis;
    measureBoundaries: (scroller: HTMLElement) => number[];
    /** Runs once scrolling is idle right before snapping so infinite buffers can recenter first. */
    onBeforeSnap?: () => void;
    /** Runs right before a smooth snap starts so callers avoid recentering mid-snap. */
    onSnapStart?: () => void;
}

/**
 * Scroll offset placing the boundary nearest to `scrollPos` at the viewport
 * origin. Boundaries are measured relative to the first item and clamped to
 * `[0, maxScroll]`.
 */
export function nearestBoundaryTarget(
    scrollPos: number,
    boundaries: readonly number[],
    maxScroll: number,
): number {
    const max = Math.max(0, maxScroll);
    if (boundaries.length === 0) return Math.min(Math.max(scrollPos, 0), max);

    const origin = boundaries[0];
    let best = 0;
    let bestDistance = Number.POSITIVE_INFINITY;
    for (const boundary of boundaries) {
        const target = Math.min(Math.max(boundary - origin, 0), max);
        const distance = Math.abs(target - scrollPos);
        if (distance < bestDistance) {
            best = target;
            bestDistance = distance;
        }
    }
    return best;
}

function readScroll(scroller: HTMLElement, axis: SnapAxis): { pos: number; max: number } {
    if (axis === 'left') {
        return { pos: scroller.scrollLeft, max: scroller.scrollWidth - scroller.clientWidth };
    }
    return { pos: scroller.scrollTop, max: scroller.scrollHeight - scroller.clientHeight };
}

/** Smoothly aligns `scroller` on `axis` to the nearest measured boundary. */
export function snapAxisScroll(
    scroller: HTMLElement,
    axis: SnapAxis,
    boundaries: readonly number[],
): boolean {
    const { pos, max } = readScroll(scroller, axis);
    const target = nearestBoundaryTarget(pos, boundaries, max);
    if (Math.abs(target - pos) < SNAP_EPSILON_PX) return false;
    if (typeof scroller.scrollTo === 'function') {
        scroller.scrollTo({ [axis]: target, behavior: 'smooth' });
    } else if (axis === 'left') {
        scroller.scrollLeft = target;
    } else {
        scroller.scrollTop = target;
    }
    return true;
}

function supportsScrollEnd(el: HTMLElement): boolean {
    return 'onscrollend' in el;
}

class AxisSnapController {
    private snapping = false;
    private lastWheelAt = Number.NEGATIVE_INFINITY;
    private settleTimer: ReturnType<typeof setTimeout> | undefined;

    constructor(
        private scroller: HTMLElement,
        private options: AttachAxisSnapOptions,
    ) {}

    private clearTimer(): void {
        if (this.settleTimer !== undefined) {
            clearTimeout(this.settleTimer);
            this.settleTimer = undefined;
        }
    }

    private schedule(delayMs: number): void {
        this.clearTimer();
        this.settleTimer = setTimeout(() => {
            this.settleTimer = undefined;
            this.performSnap();
        }, delayMs);
    }

    private performSnap(): void {
        this.clearTimer();
        if (this.snapping) {
            this.snapping = false;
            return;
        }
        this.options.onBeforeSnap?.();
        const { pos, max } = readScroll(this.scroller, this.options.axis);
        const boundaries = this.options.measureBoundaries(this.scroller);
        const target = nearestBoundaryTarget(pos, boundaries, max);
        if (Math.abs(target - pos) < SNAP_EPSILON_PX) return;

        this.options.onSnapStart?.();
        this.snapping = typeof this.scroller.scrollTo === 'function';
        snapAxisScroll(this.scroller, this.options.axis, boundaries);
    }

    private onWheel = (): void => {
        this.snapping = false;
        this.lastWheelAt = performance.now();
        this.clearTimer();
    };

    private onPointerInput = (): void => {
        this.snapping = false;
        this.clearTimer();
    };

    private onScrollActivity = (): void => {
        if (!this.snapping && this.settleTimer !== undefined) {
            this.schedule(WHEEL_SETTLE_MS);
        }
    };

    private onScrollEnd = (): void => {
        if (this.snapping) {
            this.snapping = false;
            return;
        }
        const sinceWheel = performance.now() - this.lastWheelAt;
        if (sinceWheel < WHEEL_SETTLE_MS) {
            this.schedule(WHEEL_SETTLE_MS - sinceWheel);
            return;
        }
        this.performSnap();
    };

    private onScrollFallback = (): void => {
        if (!this.snapping) {
            this.schedule(IDLE_SNAP_DELAY_MS);
        }
    };

    attach(): () => void {
        const el = this.scroller;
        const hasScrollEnd = supportsScrollEnd(el);
        el.addEventListener('wheel', this.onWheel, { passive: true });
        el.addEventListener('pointerdown', this.onPointerInput, { passive: true });
        el.addEventListener('touchstart', this.onPointerInput, { passive: true });

        if (hasScrollEnd) {
            el.addEventListener('scroll', this.onScrollActivity, { passive: true });
            el.addEventListener('scrollend', this.onScrollEnd);
        } else {
            el.addEventListener('scroll', this.onScrollFallback);
        }

        return () => {
            this.clearTimer();
            if (hasScrollEnd) {
                el.removeEventListener('scroll', this.onScrollActivity);
                el.removeEventListener('scrollend', this.onScrollEnd);
            } else {
                el.removeEventListener('scroll', this.onScrollFallback);
            }
            el.removeEventListener('wheel', this.onWheel);
            el.removeEventListener('pointerdown', this.onPointerInput);
            el.removeEventListener('touchstart', this.onPointerInput);
        };
    }
}

/**
 * Aligns scrolling onto item boundaries once momentum genuinely settles.
 * Defers snapping while wheel/trackpad momentum events are arriving so
 * high-momentum flings never bounce against premature snaps, and suppresses
 * its own completion `scrollend` to prevent oscillation.
 */
export function attachAxisSnap(
    scroller: HTMLElement,
    options: AttachAxisSnapOptions,
): () => void {
    return new AxisSnapController(scroller, options).attach();
}
