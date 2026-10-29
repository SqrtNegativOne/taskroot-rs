import type { EventInstance } from '../../../lib/domain';
import { addDays, dayDiff, MS_PER_MINUTE, parseLocalYmd, ymd } from '../../../lib/time';
import { HOURS_PER_DAY, MINUTES_IN_HOUR, PX_PER_MIN, SNAP_MIN } from '../day-timeline/constants';

export const DEFAULT_DROP_DURATION_MINUTES = 60;
export const MINUTES_PER_DAY = HOURS_PER_DAY * MINUTES_IN_HOUR;

export interface ReschedulePayload {
    startTime: string;
    endTime: string;
    isAllDay: boolean;
}

/** Snap a pixel offset within a timeline column to a valid start minute. */
export function snappedDropMinute(offsetPx: number, durationMins: number): number {
    const fitted = Math.min(Math.max(durationMins, 0), MINUTES_PER_DAY);
    const raw = Math.round(offsetPx / PX_PER_MIN / SNAP_MIN) * SNAP_MIN;
    return Math.max(0, Math.min(MINUTES_PER_DAY - fitted, raw));
}

/**
 * Payload for dropping an instance onto a date-grid cell.
 *
 * All-day instances keep their floating date range and are shifted by whole
 * days; timed instances keep their local time-of-day and duration, only the
 * date changes.
 */
export function gridDayDropPayload(event: EventInstance, date: string): ReschedulePayload {
    if (event.timing.kind === 'allDay') {
        const shift = dayDiff(parseLocalYmd(date), parseLocalYmd(event.timing.startDate));
        return {
            startTime: date,
            endTime: ymd(addDays(parseLocalYmd(event.timing.endDateExclusive), shift)),
            isAllDay: true,
        };
    }

    const start = new Date(event.timing.start);
    const duration = new Date(event.timing.end).getTime() - start.getTime();
    const newStart = parseLocalYmd(date);
    newStart.setHours(start.getHours(), start.getMinutes(), 0, 0);
    return {
        startTime: newStart.toISOString(),
        endTime: new Date(newStart.getTime() + duration).toISOString(),
        isAllDay: false,
    };
}

/**
 * Payload for dropping an instance onto a timeline column at `minute`.
 *
 * A timed instance keeps its duration; an all-day instance becomes a timed
 * block with the default one-hour duration.
 */
export function timelineDropPayload(
    event: EventInstance,
    date: string,
    minute: number,
): ReschedulePayload {
    const rawDuration =
        event.timing.kind === 'allDay'
            ? DEFAULT_DROP_DURATION_MINUTES
            : Math.round(
                  (new Date(event.timing.end).getTime() - new Date(event.timing.start).getTime()) /
                      MS_PER_MINUTE,
              );
    // A single-day timeline cannot hold a longer-than-day block; clamp it so the
    // drop and its preview stay inside the column instead of overflowing.
    const duration = Math.min(Math.max(rawDuration, SNAP_MIN), MINUTES_PER_DAY);

    const start = parseLocalYmd(date);
    start.setMinutes(minute);
    return {
        startTime: start.toISOString(),
        endTime: new Date(start.getTime() + duration * MS_PER_MINUTE).toISOString(),
        isAllDay: false,
    };
}
