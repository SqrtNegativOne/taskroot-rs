import { addDays } from '../../lib/time';

export const WRAP_START_HOUR = 22;
export const WRAP_END_HOUR = 3;

const wrapHeaderDateFormat = new Intl.DateTimeFormat('en-US', {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
});

function localMidnight(date: Date): Date {
    return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

/**
 * Whether the evening wrap window is open (10:00 PM through 2:59:59 AM local time).
 */
export function isWrapWindowOpen(date: Date): boolean {
    const hour = date.getHours();
    return hour >= WRAP_START_HOUR || hour < WRAP_END_HOUR;
}

/**
 * Whether `date` falls in the post-midnight portion of the wrap window (00:00-02:59).
 */
export function isPostMidnightWrap(date: Date): boolean {
    return date.getHours() < WRAP_END_HOUR;
}

/**
 * Local calendar day being wrapped at `date`: after midnight (before 3 AM)
 * resolves to yesterday so late-night wrap-ups still review the day ending.
 */
export function wrapTargetDate(date: Date): Date {
    const today = localMidnight(date);
    return isPostMidnightWrap(date) ? addDays(today, -1) : today;
}

/** Local midnight of the day following `wrapDate`. */
export function tomorrowOf(wrapDate: Date): Date {
    return addDays(localMidnight(wrapDate), 1);
}

/** Short local date header such as `"Thu, Nov 26"`. */
export function formatWrapDateLabel(date: Date): string {
    return wrapHeaderDateFormat.format(date);
}
