import type { EventInstance } from '../../../lib/domain';
import { addDays, dayDiff, ymd } from '../../../lib/time';
import { HOURS_PER_DAY, MINUTES_IN_HOUR, PX_PER_MIN, SNAP_MIN } from '../day-timeline/constants';

export const DEFAULT_DROP_DURATION_MINUTES = 60;
const MS_PER_MINUTE = 60_000;
const MINUTES_PER_DAY = HOURS_PER_DAY * MINUTES_IN_HOUR;

export interface ReschedulePayload {
    startTime: string;
    endTime: string;
    isAllDay: boolean;
}

function parseYmd(value: string): Date {
    return new Date(`${value}T00:00:00`);
}

/** Snap a pixel offset within a timeline column to a valid start minute. */
export function snappedDropMinute(offsetPx: number, durationMins: number): number {
    const raw = Math.round(offsetPx / PX_PER_MIN / SNAP_MIN) * SNAP_MIN;
    return Math.max(0, Math.min(MINUTES_PER_DAY - durationMins, raw));
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
        const shift = dayDiff(parseYmd(date), parseYmd(event.timing.startDate));
        return {
            startTime: date,
            endTime: ymd(addDays(parseYmd(event.timing.endDateExclusive), shift)),
            isAllDay: true,
        };
    }

    const start = new Date(event.timing.start);
    const duration = new Date(event.timing.end).getTime() - start.getTime();
    const newStart = parseYmd(date);
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
    const duration =
        event.timing.kind === 'allDay'
            ? DEFAULT_DROP_DURATION_MINUTES
            : Math.round(
                  (new Date(event.timing.end).getTime() - new Date(event.timing.start).getTime()) /
                      MS_PER_MINUTE,
              );

    const start = parseYmd(date);
    start.setMinutes(minute);
    return {
        startTime: start.toISOString(),
        endTime: new Date(start.getTime() + duration * MS_PER_MINUTE).toISOString(),
        isAllDay: false,
    };
}
