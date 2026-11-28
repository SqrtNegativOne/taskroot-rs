import {
    differenceInCalendarDays,
    getISOWeek,
    getISOWeeksInYear,
    getISOWeekYear,
    isSameDay,
    startOfWeek as startOfWeekFns,
} from 'date-fns';

export { addDays, addMonths, startOfMonth } from 'date-fns';

export const MS_PER_MINUTE = 60_000;

export function ymd(date: Date): string {
    const year = date.getFullYear().toString().padStart(4, '0');
    const month = (date.getMonth() + 1).toString().padStart(2, '0');
    const day = date.getDate().toString().padStart(2, '0');
    return `${year}-${month}-${day}`;
}

/** Local midnight of a `YYYY-MM-DD` date string. */
export function parseLocalYmd(value: string): Date {
    return new Date(`${value}T00:00:00`);
}

export function minutesSinceMidnight(date: Date): number {
    return date.getHours() * 60 + date.getMinutes();
}

/** Zero-padded 24-hour clock time (`HH:MM`) from minutes since local midnight. */
export function formatHhMm(mins: number): string {
    const minute = Math.round(mins);
    const h = Math.floor(minute / 60).toString().padStart(2, '0');
    const m = (minute % 60).toString().padStart(2, '0');
    return `${h}:${m}`;
}

const EXPLICIT_OFFSET_RE = /(?:[Zz]|[+-]\d{2}:?\d{2})$/;
const DATE_ONLY_RE = /^(\d{4}-\d{2}-\d{2})$/;
const NAIVE_DATE_TIME_RE = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})/;

export interface WallClockParts {
    date: string;
    time: string;
}

/** Formats a local `Date` as canonical wall-clock `YYYY-MM-DDTHH:MM:SS`. */
export function formatLocalDateTime(date: Date): string {
    const hh = date.getHours().toString().padStart(2, '0');
    const mm = date.getMinutes().toString().padStart(2, '0');
    const ss = date.getSeconds().toString().padStart(2, '0');
    return `${ymd(date)}T${hh}:${mm}:${ss}`;
}

/** Whether an ISO date-time string carries an explicit UTC (`Z`) or numeric offset. */
export function hasExplicitOffset(value?: string | null): boolean {
    return Boolean(value && EXPLICIT_OFFSET_RE.test(value));
}

/** Whether a timezone identifier denotes UTC/GMT rather than a local IANA zone. */
export function isUtcZone(timezone?: string | null): boolean {
    if (!timezone) return false;
    const upper = timezone.trim().toUpperCase();
    return upper === 'UTC' || upper === 'ETC/UTC' || upper === 'GMT' || upper === 'Z';
}

function formatInstantInZone(instant: Date, timeZone: string): WallClockParts | undefined {
    try {
        const parts = new Intl.DateTimeFormat('en-US', {
            timeZone,
            year: 'numeric',
            month: '2-digit',
            day: '2-digit',
            hour: '2-digit',
            minute: '2-digit',
            hourCycle: 'h23',
        }).formatToParts(instant);
        const byType = new Map(parts.map((part) => [part.type, part.value]));
        const year = byType.get('year');
        const month = byType.get('month');
        const day = byType.get('day');
        const hour = byType.get('hour');
        const minute = byType.get('minute');
        if (!year || !month || !day || !hour || !minute) return undefined;
        const hh = hour === '24' ? '00' : hour;
        return { date: `${year}-${month}-${day}`, time: `${hh}:${minute}` };
    } catch {
        return undefined;
    }
}

/**
 * Extracts wall-clock `{ date: 'YYYY-MM-DD', time: 'HH:MM' | '' }` from a
 * canonical `CalendarDateTime` string.
 *
 * - Date-only `YYYY-MM-DD` stays floating with `time: ''`.
 * - Naive `YYYY-MM-DDTHH:MM...` is already wall-clock and keeps its digits.
 * - UTC (`...Z`) or offset (`±HH:MM`) instants convert into `timezone` when a
 *   non-UTC IANA zone is given, or into the host's local timezone otherwise so
 *   CalDAV `...Z` timestamps never display raw UTC digits.
 */
export function toWallClockParts(
    iso?: string | null,
    timezone?: string | null,
): WallClockParts | undefined {
    if (!iso) return undefined;
    const dateOnly = DATE_ONLY_RE.exec(iso);
    if (dateOnly) return { date: dateOnly[1]!, time: '' };

    if (EXPLICIT_OFFSET_RE.test(iso)) {
        const instant = new Date(iso);
        if (Number.isNaN(instant.getTime())) return undefined;
        if (timezone && !isUtcZone(timezone)) {
            const zoned = formatInstantInZone(instant, timezone);
            if (zoned) return zoned;
        }
        const hh = instant.getHours().toString().padStart(2, '0');
        const mm = instant.getMinutes().toString().padStart(2, '0');
        return { date: ymd(instant), time: `${hh}:${mm}` };
    }

    const naive = NAIVE_DATE_TIME_RE.exec(iso);
    if (naive) return { date: naive[1]!, time: naive[2]! };
    return undefined;
}

export function dayDiff(a: Date, b: Date): number {
    return differenceInCalendarDays(a, b);
}

export function sameDay(a: Date, b: Date): boolean {
    return isSameDay(a, b);
}

/** Monday 00:00 of the week containing `date`, in local time. */
export function startOfWeek(date: Date): Date {
    return startOfWeekFns(date, { weekStartsOn: 1 });
}

/** ISO-8601 week number (weeks start on Monday, week 1 contains the first Thursday). */
export function getWeekNumber(date: Date): number {
    return getISOWeek(date);
}

/** Number of ISO-8601 weeks in `year` (52 or 53). */
export function isoWeeksInYear(year: number): number {
    return getISOWeeksInYear(new Date(year, 11, 28));
}

/** The ISO week-year containing `date` (can differ from its calendar year around New Year). */
export function isoWeekYear(date: Date): number {
    return getISOWeekYear(date);
}
