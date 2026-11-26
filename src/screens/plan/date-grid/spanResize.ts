import type { EventInstance } from '../../../lib/domain';
import { addDays, dayDiff, parseLocalYmd, ymd } from '../../../lib/time';
import { DAYS_IN_WEEK } from './constants';

export type SpanEdge = 'start' | 'end';

export interface AllDayRange {
    startDate: string;
    endDateExclusive: string;
}

export interface SpanColumns {
    startCol: number;
    endColExclusive: number;
}

function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
}

/**
 * The all-day range after dragging one edge of a spanning bar onto
 * `candidateDate`. Both edges stay on whole days and the range never collapses
 * below one day; timed events cannot be resized this way.
 */
export function resizeAllDayRange(
    instance: EventInstance,
    edge: SpanEdge,
    candidateDate: string,
): AllDayRange | undefined {
    if (instance.timing.kind !== 'allDay') return undefined;

    const { startDate, endDateExclusive } = instance.timing;
    if (edge === 'start') {
        const lastStart = ymd(addDays(parseLocalYmd(endDateExclusive), -1));
        return {
            startDate: candidateDate > lastStart ? lastStart : candidateDate,
            endDateExclusive,
        };
    }

    const firstEndExclusive = ymd(addDays(parseLocalYmd(startDate), 1));
    const candidateEndExclusive = ymd(addDays(parseLocalYmd(candidateDate), 1));
    return {
        startDate,
        endDateExclusive:
            candidateEndExclusive < firstEndExclusive
                ? firstEndExclusive
                : candidateEndExclusive,
    };
}

/** Map a resized range to the columns of the week starting on `weekStartDate`. */
export function previewColumns(weekStartDate: string, range: AllDayRange): SpanColumns {
    const weekStart = parseLocalYmd(weekStartDate);
    const startCol = clamp(
        dayDiff(parseLocalYmd(range.startDate), weekStart),
        0,
        DAYS_IN_WEEK - 1,
    );
    const endColExclusive = clamp(
        dayDiff(parseLocalYmd(range.endDateExclusive), weekStart),
        startCol + 1,
        DAYS_IN_WEEK,
    );
    return { startCol, endColExclusive };
}
