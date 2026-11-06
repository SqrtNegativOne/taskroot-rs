import type { AppEvent } from '../../lib/domain';
import { addDays, dayDiff, parseLocalYmd, ymd } from '../../lib/time';
import { getFormattedTime } from './format';

export type EventDateFields = Pick<AppEvent, 'startTime' | 'endTime' | 'isAllDay'>;

const DATE_LENGTH = 10;
const DEFAULT_START_TIME = '09:00';
const DEFAULT_DURATION_MINUTES = 60;

function datePart(value?: string): string | undefined {
    const part = value?.slice(0, DATE_LENGTH);
    return part && /^\d{4}-\d{2}-\d{2}$/.test(part) ? part : undefined;
}

function timePart(value?: string): string | undefined {
    if (!value?.includes('T')) return undefined;
    return getFormattedTime(value) || undefined;
}

function shiftDay(date: string, days: number): string {
    return ymd(addDays(parseLocalYmd(date), days));
}

function addMinutes(time: string, minutes: number): string {
    const [hours = 0, mins = 0] = time.split(':').map(Number);
    const total = (hours * 60 + mins + minutes + 1440) % 1440;
    return `${Math.floor(total / 60).toString().padStart(2, '0')}:${(total % 60).toString().padStart(2, '0')}`;
}

function isBefore(
    dateA: string,
    timeA: string,
    dateB: string,
    timeB: string,
): boolean {
    return `${dateA}T${timeA}` < `${dateB}T${timeB}`;
}

export function eventIsAllDay(event: Pick<AppEvent, 'startTime' | 'isAllDay'>): boolean {
    return event.isAllDay ?? !event.startTime.includes('T');
}

export function eventStartDate(event: Pick<AppEvent, 'startTime'>): string | undefined {
    return datePart(event.startTime);
}

/** The inclusive end day the UI shows for an all-day event. */
export function eventEndDateInclusive(event: Pick<AppEvent, 'startTime' | 'endTime'>): string | undefined {
    const start = datePart(event.startTime);
    const endExclusive = datePart(event.endTime);
    if (!start) return undefined;
    if (!endExclusive || endExclusive <= start) return start;
    const inclusive = shiftDay(endExclusive, -1);
    return inclusive < start ? start : inclusive;
}

/** The calendar date an event ends on (inclusive for all-day, wall date for timed). */
export function eventEndDate(event: Pick<AppEvent, 'startTime' | 'endTime'>): string | undefined {
    return datePart(event.endTime) ?? datePart(event.startTime);
}

export function toAllDay(event: AppEvent): EventDateFields {
    const start = datePart(event.startTime) ?? ymd(new Date());
    const end = datePart(event.endTime);

    if (eventIsAllDay(event)) {
        const endExclusive = end && end > start ? end : shiftDay(start, 1);
        return { startTime: start, endTime: endExclusive, isAllDay: true };
    }

    const endTime = timePart(event.endTime) ?? DEFAULT_START_TIME;
    const inclusiveEnd = end && end > start ? end : start;
    const lastDay = endTime === '00:00' && inclusiveEnd > start
        ? shiftDay(inclusiveEnd, -1)
        : inclusiveEnd;
    return { startTime: start, endTime: shiftDay(lastDay, 1), isAllDay: true };
}

export function toTimed(event: AppEvent): EventDateFields {
    const startDate = datePart(event.startTime) ?? ymd(new Date());
    const startTime = timePart(event.startTime) ?? DEFAULT_START_TIME;
    const endDate = eventIsAllDay(event)
        ? eventEndDateInclusive(event) ?? startDate
        : datePart(event.endTime) ?? startDate;
    const endTime = timePart(event.endTime) ?? addMinutes(startTime, DEFAULT_DURATION_MINUTES);

    if (!isBefore(startDate, startTime, endDate, endTime)) {
        return {
            startTime: `${startDate}T${startTime}:00`,
            endTime: `${startDate}T${addMinutes(startTime, DEFAULT_DURATION_MINUTES)}:00`,
            isAllDay: false,
        };
    }

    return {
        startTime: `${startDate}T${startTime}:00`,
        endTime: `${endDate}T${endTime}:00`,
        isAllDay: false,
    };
}

export function withStartDate(event: AppEvent, date: string): EventDateFields {
    const oldStart = datePart(event.startTime) ?? date;
    const delta = dayDiff(parseLocalYmd(date), parseLocalYmd(oldStart));
    const end = datePart(event.endTime);

    if (eventIsAllDay(event)) {
        const endExclusive = end ? shiftDay(end, delta) : shiftDay(date, 1);
        return {
            startTime: date,
            endTime: endExclusive > date ? endExclusive : shiftDay(date, 1),
            isAllDay: true,
        };
    }

    const startTime = timePart(event.startTime) ?? DEFAULT_START_TIME;
    const endTime = timePart(event.endTime) ?? addMinutes(startTime, DEFAULT_DURATION_MINUTES);
    const movedEndDate = end ? shiftDay(end, delta) : date;
    const endDate = isBefore(date, startTime, movedEndDate, endTime) ? movedEndDate : date;
    const resolvedEndTime = isBefore(date, startTime, endDate, endTime)
        ? endTime
        : addMinutes(startTime, DEFAULT_DURATION_MINUTES);

    return {
        startTime: `${date}T${startTime}:00`,
        endTime: `${endDate}T${resolvedEndTime}:00`,
        isAllDay: false,
    };
}

export function withStartTime(event: AppEvent, time: string): EventDateFields {
    const date = datePart(event.startTime) ?? ymd(new Date());
    const endDate = datePart(event.endTime) ?? date;
    const endTime = timePart(event.endTime) ?? addMinutes(time, DEFAULT_DURATION_MINUTES);

    if (!isBefore(date, time, endDate, endTime)) {
        return {
            startTime: `${date}T${time}:00`,
            endTime: `${date}T${addMinutes(time, DEFAULT_DURATION_MINUTES)}:00`,
            isAllDay: false,
        };
    }

    return {
        startTime: `${date}T${time}:00`,
        endTime: `${endDate}T${endTime}:00`,
        isAllDay: false,
    };
}

export function withEndDate(event: AppEvent, date: string): EventDateFields {
    const start = datePart(event.startTime) ?? date;

    if (eventIsAllDay(event)) {
        const inclusive = date && date > start ? date : start;
        return { startTime: start, endTime: shiftDay(inclusive, 1), isAllDay: true };
    }

    const startTime = timePart(event.startTime) ?? DEFAULT_START_TIME;
    const endTime = timePart(event.endTime) ?? addMinutes(startTime, DEFAULT_DURATION_MINUTES);
    const clamped = !isBefore(start, startTime, date, endTime)
        ? shiftDay(start, 1)
        : date;
    return {
        startTime: `${start}T${startTime}:00`,
        endTime: `${clamped}T${endTime}:00`,
        isAllDay: false,
    };
}

export function withEndTime(event: AppEvent, time: string): EventDateFields {
    const start = datePart(event.startTime) ?? ymd(new Date());
    const startTime = timePart(event.startTime) ?? DEFAULT_START_TIME;
    const endDate = datePart(event.endTime) ?? start;
    const clampedDate = !isBefore(start, startTime, endDate, time)
        ? shiftDay(start, 1)
        : endDate;
    return {
        startTime: `${start}T${startTime}:00`,
        endTime: `${clampedDate}T${time}:00`,
        isAllDay: false,
    };
}
