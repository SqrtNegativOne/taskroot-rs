import type { EventInstance } from '../bindings/EventInstance.generated';
import { addDays, ymd } from '../time';

/**
 * Whether an instance is visible on the given local calendar day.
 *
 * All-day events compare floating `YYYY-MM-DD` strings (never an instant);
 * timed events compare their UTC instants against the wall-clock day
 * boundaries (`d` through `d+1`), so an event lands on the day the user
 * actually sees — exact even on 23/25-hour DST days.
 */
export function instanceOccursOnDay(event: EventInstance, date: Date): boolean {
    const timing = event.timing;
    if (timing.kind === 'allDay') {
        const day = ymd(date);
        return timing.startDate <= day && timing.endDateExclusive > day;
    }
    const dayStart = new Date(date.getFullYear(), date.getMonth(), date.getDate());
    const dayEnd = addDays(dayStart, 1);
    return new Date(timing.start) < dayEnd && new Date(timing.end) > dayStart;
}
