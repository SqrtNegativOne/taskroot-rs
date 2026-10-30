import { instanceOccursOnDay } from '../../../lib/domain';
import type { EventInstance } from '../../../lib/domain';
import { minutesSinceMidnight, ymd } from '../../../lib/time';
import { MINUTES_PER_DAY } from '../snapping';
import type { UnlaidEvent } from './types';

function minutesIntoDay(time: Date, dayStart: Date, dayEnd: Date): number {
    if (time <= dayStart) return 0;
    if (time >= dayEnd) return MINUTES_PER_DAY;
    return minutesSinceMidnight(time);
}

/**
 * Assign each timed instance to the local days it overlaps.
 *
 * All-day instances are skipped here; they belong to the date grid, which
 * compares floating dates. Timed instances are bucketed by instants against
 * local day boundaries, never by string prefix on a raw timestamp.
 */
export function bucketEventsByDay(
    events: readonly EventInstance[],
    dates: readonly Date[],
): Record<string, UnlaidEvent[]> {
    const bucketed: Record<string, UnlaidEvent[]> = {};

    for (const ev of events) {
        if (ev.timing.kind !== 'timed') continue;

        const start = new Date(ev.timing.start);
        const end = new Date(ev.timing.end);
        if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) continue;

        for (const date of dates) {
            if (!instanceOccursOnDay(ev, date)) continue;

            const dayStart = new Date(date.getFullYear(), date.getMonth(), date.getDate());
            const dayEnd = new Date(date.getFullYear(), date.getMonth(), date.getDate() + 1);

            const dateStr = ymd(date);
            bucketed[dateStr] ??= [];
            bucketed[dateStr].push({
                event: ev,
                startMins: minutesIntoDay(start, dayStart, dayEnd),
                endMins: minutesIntoDay(end, dayStart, dayEnd),
            });
        }
    }

    return bucketed;
}
