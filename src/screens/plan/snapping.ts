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

/** Resolve the visible first hour (0..23) from wake-up minutes or an explicit hour override. */
export function resolveStartHour(
    overrideHour: number | undefined,
    wakeUpMinutes: number | undefined,
): number {
    if (overrideHour !== undefined) {
        const hour = Number.isFinite(overrideHour) ? Math.floor(overrideHour) : 0;
        return Math.min(HOURS_PER_DAY - 1, Math.max(0, hour));
    }
    const raw = wakeUpMinutes !== undefined && Number.isFinite(wakeUpMinutes) ? wakeUpMinutes : 0;
    const clamped = Math.min((HOURS_PER_DAY - 1) * MINUTES_IN_HOUR, Math.max(0, raw));
    return Math.floor(clamped / MINUTES_IN_HOUR);
}

/** Snap a pixel offset within a column starting at `startMinute` to a valid drop start minute. */
export function snappedDropMinuteFrom(
    offsetPx: number,
    durationMins: number,
    startMinute = 0,
): number {
    const fitted = Math.min(Math.max(durationMins, 0), MINUTES_PER_DAY);
    return Math.max(
        startMinute,
        Math.min(MINUTES_PER_DAY - fitted, startMinute + snapPixels(offsetPx)),
    );
}
