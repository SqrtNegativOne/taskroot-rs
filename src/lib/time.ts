const MS_PER_DAY = 86_400_000;
const MS_PER_WEEK = 604_800_000;

export function ymd(date: Date): string {
    const year = date.getFullYear().toString().padStart(4, '0');
    const month = (date.getMonth() + 1).toString().padStart(2, '0');
    const day = date.getDate().toString().padStart(2, '0');
    return `${year}-${month}-${day}`;
}

export function minutesSinceMidnight(date: Date): number {
    return date.getHours() * 60 + date.getMinutes();
}

export function addDays(date: Date, days: number): Date {
    const next = new Date(date);
    next.setDate(next.getDate() + days);
    return next;
}

export function dayDiff(a: Date, b: Date): number {
    const utcA = Date.UTC(a.getFullYear(), a.getMonth(), a.getDate());
    const utcB = Date.UTC(b.getFullYear(), b.getMonth(), b.getDate());
    return Math.round((utcA - utcB) / MS_PER_DAY);
}

export function sameDay(a: Date, b: Date): boolean {
    return ymd(a) === ymd(b);
}

/** Monday 00:00 of the week containing `date`, in local time. */
export function startOfWeek(date: Date): Date {
    const start = new Date(date);
    const day = start.getDay();
    const diff = start.getDate() - day + (day === 0 ? -6 : 1);
    start.setDate(diff);
    start.setHours(0, 0, 0, 0);
    return start;
}

/** First day of the month containing `date`, at local midnight. */
export function startOfMonth(date: Date): Date {
    return new Date(date.getFullYear(), date.getMonth(), 1);
}

/** ISO-8601 week number (weeks start on Monday, week 1 contains the first Thursday). */
export function getWeekNumber(date: Date): number {
    const target = new Date(date.valueOf());
    const dayNr = (date.getDay() + 6) % 7;
    target.setDate(target.getDate() - dayNr + 3);
    const firstThursday = target.valueOf();
    target.setMonth(0, 1);
    if (target.getDay() !== 4) {
        target.setMonth(0, 1 + ((4 - target.getDay()) + 7) % 7);
    }
    return 1 + Math.ceil((firstThursday - target.valueOf()) / MS_PER_WEEK);
}

/** Number of ISO-8601 weeks in `year` (52 or 53). */
export function isoWeeksInYear(year: number): number {
    return getWeekNumber(new Date(year, 11, 28));
}

/** The ISO week-year containing `date` (can differ from its calendar year around New Year). */
export function isoWeekYear(date: Date): number {
    const thursday = addDays(startOfWeek(date), 3);
    return thursday.getFullYear();
}
