import type { AppEvent } from '../../lib/domain';
import { addDays, dayDiff, parseLocalYmd, ymd } from '../../lib/time';
import { formatFriendlyDate, getFormattedTime } from './format';

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

function addMinutes(time: string, minutes: number): { time: string; dayOffset: number } {
    const [hours = 0, mins = 0] = time.split(':').map(Number);
    const total = hours * 60 + mins + minutes;
    const dayOffset = Math.floor(total / 1440);
    const rem = ((total % 1440) + 1440) % 1440;
    const hh = Math.floor(rem / 60).toString().padStart(2, '0');
    const mm = (rem % 60).toString().padStart(2, '0');
    return { time: `${hh}:${mm}`, dayOffset };
}

function isBefore(
    dateA: string,
    timeA: string,
    dateB: string,
    timeB: string,
): boolean {
    return `${dateA}T${timeA}` < `${dateB}T${timeB}`;
}

function timedRange(
    startDate: string,
    startTime: string,
    endDate: string,
    endTime: string | undefined,
): EventDateFields {
    if (endTime !== undefined && isBefore(startDate, startTime, endDate, endTime)) {
        return {
            startTime: `${startDate}T${startTime}:00`,
            endTime: `${endDate}T${endTime}:00`,
            isAllDay: false,
        };
    }
    const { time, dayOffset } = addMinutes(startTime, DEFAULT_DURATION_MINUTES);
    const resolvedEndDate = dayOffset === 0 ? startDate : shiftDay(startDate, dayOffset);
    return {
        startTime: `${startDate}T${startTime}:00`,
        endTime: `${resolvedEndDate}T${time}:00`,
        isAllDay: false,
    };
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

export function toAllDay(event: EventDateFields): EventDateFields {
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

export function toTimed(event: EventDateFields): EventDateFields {
    const startDate = datePart(event.startTime) ?? ymd(new Date());
    const startTime = timePart(event.startTime) ?? DEFAULT_START_TIME;
    const endDate = eventIsAllDay(event)
        ? eventEndDateInclusive(event) ?? startDate
        : datePart(event.endTime) ?? startDate;
    const parsedEnd = timePart(event.endTime);
    const fallback = addMinutes(startTime, DEFAULT_DURATION_MINUTES);
    const candidateEndDate = parsedEnd !== undefined || fallback.dayOffset === 0
        ? endDate
        : shiftDay(endDate, fallback.dayOffset);
    return timedRange(startDate, startTime, candidateEndDate, parsedEnd ?? fallback.time);
}

export function withStartDate(event: EventDateFields, date: string): EventDateFields {
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
    const movedEndDate = end ? shiftDay(end, delta) : date;
    return timedRange(date, startTime, movedEndDate, timePart(event.endTime));
}

export function withStartTime(event: EventDateFields, time: string): EventDateFields {
    const date = datePart(event.startTime) ?? ymd(new Date());
    const endDate = datePart(event.endTime) ?? date;
    return timedRange(date, time, endDate, timePart(event.endTime));
}

export function withEndDate(event: EventDateFields, date: string): EventDateFields {
    const start = datePart(event.startTime) ?? date;

    if (eventIsAllDay(event)) {
        const inclusive = date && date > start ? date : start;
        return { startTime: start, endTime: shiftDay(inclusive, 1), isAllDay: true };
    }

    const startTime = timePart(event.startTime) ?? DEFAULT_START_TIME;
    const endTime = timePart(event.endTime) ?? addMinutes(startTime, DEFAULT_DURATION_MINUTES).time;
    const minEndDate = endTime > startTime ? start : shiftDay(start, 1);
    const clamped = date < minEndDate ? minEndDate : date;
    return {
        startTime: `${start}T${startTime}:00`,
        endTime: `${clamped}T${endTime}:00`,
        isAllDay: false,
    };
}

export function withEndTime(event: EventDateFields, time: string): EventDateFields {
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

/** Update both start and inclusive end dates, preserving all-day vs timed times cleanly. */
export function withDateRange(
    event: EventDateFields,
    startYmd: string,
    endYmd: string,
): EventDateFields {
    const [start, end] = startYmd <= endYmd ? [startYmd, endYmd] : [endYmd, startYmd];
    if (eventIsAllDay(event)) {
        return { startTime: start, endTime: shiftDay(end, 1), isAllDay: true };
    }

    const startTime = timePart(event.startTime) ?? DEFAULT_START_TIME;
    const endTime = timePart(event.endTime) ?? addMinutes(startTime, DEFAULT_DURATION_MINUTES).time;
    if (isBefore(start, startTime, end, endTime)) {
        return { startTime: `${start}T${startTime}:00`, endTime: `${end}T${endTime}:00`, isAllDay: false };
    }

    const plusHour = addMinutes(startTime, DEFAULT_DURATION_MINUTES);
    const resolvedEnd = plusHour.dayOffset === 0
        ? `${end}T${plusHour.time}:00`
        : startTime < '23:59' ? `${end}T23:59:00` : `${shiftDay(end, 1)}T${plusHour.time}:00`;
    return { startTime: `${start}T${startTime}:00`, endTime: resolvedEnd, isAllDay: false };
}

/** Unified readable schedule summary for an event. */
export function formatEventSchedule(event: EventDateFields): string {
    const startYmd = eventStartDate(event);
    if (!startYmd) return 'No date';
    const startLabel = formatFriendlyDate(startYmd);

    if (eventIsAllDay(event)) {
        const endYmd = eventEndDateInclusive(event) ?? startYmd;
        return endYmd === startYmd
            ? `${startLabel} · All day`
            : `${startLabel} – ${formatFriendlyDate(endYmd)} · All day`;
    }

    const endYmd = eventEndDate(event) ?? startYmd;
    const startHm = getFormattedTime(event.startTime) || '00:00';
    const endHm = getFormattedTime(event.endTime) || startHm;
    if (endYmd === startYmd) return `${startLabel} · ${startHm} – ${endHm}`;
    return `${startLabel}, ${startHm} – ${formatFriendlyDate(endYmd)}, ${endHm}`;
}
