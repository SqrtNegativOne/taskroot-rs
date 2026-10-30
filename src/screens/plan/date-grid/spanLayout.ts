import type { EventInstance } from '../../../lib/domain';
import { addDays, dayDiff, parseLocalYmd, ymd } from '../../../lib/time';
import { DAYS_IN_WEEK } from './constants';

/** One week-clipped segment of a multi-day all-day instance. */
export interface WeekSpan {
    instance: EventInstance;
    occurrenceKey: string;
    /** Inclusive start column within the week (0 = Monday). */
    startCol: number;
    /** Exclusive end column within the week (1..7). */
    endColExclusive: number;
    /** Row within the week's span band, assigned so segments never overlap. */
    lane: number;
    continuesBefore: boolean;
    continuesAfter: boolean;
}

export interface WeekSpanLayout {
    spans: WeekSpan[];
    laneCount: number;
}

type WeekSegment = Omit<WeekSpan, 'instance' | 'occurrenceKey' | 'lane'>;

/**
 * Whether an instance is an all-day event covering more than one calendar day.
 *
 * Single-day all-day events stay inline in their `DayCell`; only multi-day
 * ranges get a spanning bar.
 */
export function isMultiDayAllDay(event: EventInstance): boolean {
    if (event.timing.kind !== 'allDay') return false;
    return dayDiff(parseLocalYmd(event.timing.endDateExclusive), parseLocalYmd(event.timing.startDate)) > 1;
}

/** Clip a multi-day range to the week, or `undefined` when it does not overlap. */
function weekSegment(
    instance: EventInstance,
    weekStartYmd: string,
    weekEndYmd: string,
): WeekSegment | undefined {
    if (instance.timing.kind !== 'allDay') return undefined;
    const { startDate, endDateExclusive } = instance.timing;
    if (dayDiff(parseLocalYmd(endDateExclusive), parseLocalYmd(startDate)) <= 1) return undefined;

    const segStart = startDate > weekStartYmd ? startDate : weekStartYmd;
    const segEnd = endDateExclusive < weekEndYmd ? endDateExclusive : weekEndYmd;
    if (segStart >= segEnd) return undefined;

    const startCol = dayDiff(parseLocalYmd(segStart), parseLocalYmd(weekStartYmd));
    return {
        startCol,
        endColExclusive: startCol + dayDiff(parseLocalYmd(segEnd), parseLocalYmd(segStart)),
        continuesBefore: startDate < weekStartYmd,
        continuesAfter: endDateExclusive > weekEndYmd,
    };
}

/** Greedily assign each pre-sorted span the lowest lane that is free. */
function assignLanes(spans: WeekSpan[]): number {
    const laneEnds: number[] = [];
    for (const span of spans) {
        const existing = laneEnds.findIndex((end) => end <= span.startCol);
        if (existing === -1) {
            span.lane = laneEnds.length;
            laneEnds.push(span.endColExclusive);
            continue;
        }
        span.lane = existing;
        laneEnds[existing] = span.endColExclusive;
    }
    return laneEnds.length;
}

/**
 * Lay out the multi-day all-day instances overlapping the week starting at
 * `weekDays[0]`.
 *
 * Each instance is clipped to the week and packed into the lowest lane whose
 * previous segment ends before it starts. `continuesBefore`/`continuesAfter`
 * mark segments whose instance extends past the week, so the caller can square
 * off those edges and render the next week's segment as a continuation.
 */
export function layoutWeekSpans(
    events: readonly EventInstance[],
    weekDays: readonly Date[],
): WeekSpanLayout {
    const weekStart = weekDays[0];
    if (!weekStart) return { spans: [], laneCount: 0 };

    const weekStartYmd = ymd(weekStart);
    const weekEndYmd = ymd(addDays(weekStart, DAYS_IN_WEEK));

    const spans: WeekSpan[] = [];
    for (const instance of events) {
        const segment = weekSegment(instance, weekStartYmd, weekEndYmd);
        if (!segment) continue;
        spans.push({ instance, occurrenceKey: instance.occurrenceKey, ...segment, lane: 0 });
    }

    spans.sort((a, b) => a.startCol - b.startCol || b.endColExclusive - a.endColExclusive);
    return { spans, laneCount: assignLanes(spans) };
}
