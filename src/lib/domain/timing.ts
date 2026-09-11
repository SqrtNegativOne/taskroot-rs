import type { EventInstance } from '../bindings/EventInstance.generated';
import type { EventTiming } from '../bindings/EventTiming.generated';
import { ymd } from '../time';

export type AllDayTiming = Extract<EventTiming, { kind: 'allDay' }>;
export type TimedTiming = Extract<EventTiming, { kind: 'timed' }>;

const MS_PER_DAY = 86_400_000;

export const DAY_MS = MS_PER_DAY;

export function isAllDayTiming(timing: EventTiming): timing is AllDayTiming {
    return timing.kind === 'allDay';
}

export function isTimedTiming(timing: EventTiming): timing is TimedTiming {
    return timing.kind === 'timed';
}

/**
 * Whether an instance is visible on the given local calendar day.
 *
 * All-day events compare floating `YYYY-MM-DD` strings (never an instant);
 * timed events compare their UTC instants against the system-local day
 * boundaries, so an event lands on the day the user actually sees.
 */
export function instanceOccursOnDay(event: EventInstance, date: Date): boolean {
    const timing = event.timing;
    if (timing.kind === 'allDay') {
        const day = ymd(date);
        return timing.startDate <= day && timing.endDateExclusive > day;
    }
    const dayStart = new Date(date.getFullYear(), date.getMonth(), date.getDate());
    const dayEnd = new Date(dayStart.getTime() + MS_PER_DAY);
    return new Date(timing.start) < dayEnd && new Date(timing.end) > dayStart;
}

/** Start of an instance as a local `Date`, used for drag and click sizing. */
export function instanceStartDate(event: EventInstance): Date | undefined {
    if (event.timing.kind === 'timed') {
        const parsed = new Date(event.timing.start);
        return Number.isNaN(parsed.getTime()) ? undefined : parsed;
    }
    const parsed = new Date(`${event.timing.startDate}T00:00:00`);
    return Number.isNaN(parsed.getTime()) ? undefined : parsed;
}
