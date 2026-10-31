import {
    differenceInCalendarDays,
    getISOWeek,
    getISOWeeksInYear,
    getISOWeekYear,
    isSameDay,
    startOfWeek as startOfWeekFns,
} from 'date-fns';

export { addDays, startOfMonth } from 'date-fns';

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
