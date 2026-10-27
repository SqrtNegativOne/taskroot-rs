import type { EventInstance } from '../bindings/EventInstance.generated';
import { ymd } from '../time';

const MS_PER_DAY = 86_400_000;

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
