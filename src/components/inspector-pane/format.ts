import { CalendarDate } from '@internationalized/date';
import { parseLocalYmd, ymd } from '../../lib/time';

const friendlyDate = new Intl.DateTimeFormat(undefined, {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
});

const friendlyDateWithYear = new Intl.DateTimeFormat(undefined, {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
});

const friendlyTimestamp = new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
});

export function getFormattedDate(iso?: string | null): string {
    if (!iso) return '';
    return iso.split('T')[0] ?? '';
}

export function getFormattedTime(iso?: string | null): string {
    if (!iso?.includes('T')) return '';
    const t = iso.split('T')[1];
    return t?.substring(0, 5) ?? '';
}

/** Replace the date of a canonical value, keeping its time (if any). */
export function withDate(iso: string | null | undefined, date: string): string | undefined {
    if (!date) return undefined;
    const time = iso?.includes('T') ? iso.slice(iso.indexOf('T')) : '';
    return `${date}${time}`;
}

/** Replace the time of a canonical value; an empty time makes it date-only. */
export function withTime(iso: string | null | undefined, time: string): string | undefined {
    const date = iso?.split('T')[0];
    if (!date) return undefined;
    return time ? `${date}T${time}:00` : date;
}

/** Combine a `YYYY-MM-DD` date and optional `HH:MM` time into a canonical value. */
export function combineDateAndTime(date: string, time: string): string | null {
    if (!date) return null;
    return time ? `${date}T${time}:00` : date;
}

/** Parse a `YYYY-MM-DD` string into a `@internationalized/date` `CalendarDate`. */
export function parseYmdToCalendarDate(value?: string | null): CalendarDate | undefined {
    if (!value) return undefined;
    const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
    if (!match) return undefined;
    const year = Number(match[1]);
    const month = Number(match[2]);
    const day = Number(match[3]);
    const check = new Date(year, month - 1, day);
    if (
        check.getFullYear() !== year
        || check.getMonth() !== month - 1
        || check.getDate() !== day
    ) {
        return undefined;
    }
    return new CalendarDate(year, month, day);
}

/** Today's local date as a `CalendarDate`. */
export function todayCalendarDate(): CalendarDate {
    return parseYmdToCalendarDate(ymd(new Date()))!;
}

/** A short, human date such as "Mon, Nov 5". */
export function formatFriendlyDate(iso?: string | null): string {
    const date = getFormattedDate(iso);
    if (!date) return '';
    return friendlyDate.format(parseLocalYmd(date));
}

/** Unified readable task date/time summary such as "Mon, Nov 5 · 14:30". */
export function formatTaskDateTime(
    iso: string | null | undefined,
    placeholder: string,
): string {
    const date = formatFriendlyDate(iso);
    if (!date) return placeholder;
    const time = getFormattedTime(iso);
    return time ? `${date} · ${time}` : date;
}

/** A human date with its year, for read-only metadata. */
export function formatFriendlyDateWithYear(iso?: string | null): string {
    const date = getFormattedDate(iso);
    if (!date) return '';
    return friendlyDateWithYear.format(parseLocalYmd(date));
}

/** A read-only timestamp (created/modified) in local time. */
export function formatTimestamp(iso?: string | null): string {
    if (!iso) return '';
    const date = new Date(iso);
    return Number.isNaN(date.getTime()) ? iso : friendlyTimestamp.format(date);
}

/** Human-readable tracked duration from whole seconds (e.g. "1h 15m", "12m", "45s"). */
export function formatTrackedDuration(seconds: number): string {
    const total = Math.max(0, Math.round(seconds));
    if (total < 60) return `${total}s`;
    const hours = Math.floor(total / 3600);
    const minutes = Math.floor((total % 3600) / 60);
    if (hours === 0) return `${minutes}m`;
    return minutes === 0 ? `${hours}h` : `${hours}h ${minutes}m`;
}
