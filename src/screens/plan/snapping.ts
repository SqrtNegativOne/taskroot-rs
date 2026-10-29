import { HOURS_PER_DAY, MINUTES_IN_HOUR, PX_PER_MIN, SNAP_MIN } from './day-timeline/constants';

/** Total minutes in one calendar day. */
export const MINUTES_PER_DAY = HOURS_PER_DAY * MINUTES_IN_HOUR;

/** Snap a pixel offset within a timeline column to the nearest snap step. */
export function snapPixels(offsetPx: number): number {
    return Math.round(offsetPx / PX_PER_MIN / SNAP_MIN) * SNAP_MIN;
}

/** Clamp a duration so it fits one timeline column and is at least one snap step. */
export function clampDuration(mins: number): number {
    return Math.min(Math.max(mins, SNAP_MIN), MINUTES_PER_DAY);
}
