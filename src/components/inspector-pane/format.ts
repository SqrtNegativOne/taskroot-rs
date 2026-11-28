import { CalendarDate } from '@internationalized/date';
import { hasExplicitOffset, isUtcZone, parseLocalYmd, toWallClockParts, ymd } from '../../lib/time';

const friendlyDate = new Intl.DateTimeFormat(undefined, {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
});

const friendlyTimestamp = new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
});

export function getFormattedDate(iso?: string | null, timezone?: string | null): string {
    return toWallClockParts(iso, timezone)?.date ?? '';
}

export function getFormattedTime(iso?: string | null, timezone?: string | null): string {
    return toWallClockParts(iso, timezone)?.time ?? '';
}

/** Replace the date of a canonical value, keeping its wall-clock time (if any). */
export function withDate(
    iso: string | null | undefined,
    date: string,
    timezone?: string | null,
): string | undefined {
    if (!date) return undefined;
    const time = getFormattedTime(iso, timezone);
    return time ? `${date}T${time}:00` : date;
}

/** Replace the time of a canonical value; an empty time makes it date-only. */
export function withTime(
    iso: string | null | undefined,
    time: string,
    timezone?: string | null,
): string | undefined {
    const date = getFormattedDate(iso, timezone);
    if (!date) return undefined;
    return time ? `${date}T${time}:00` : date;
}

/** Combine a `YYYY-MM-DD` date and optional `HH:MM` time into a canonical value. */
export function combineDateAndTime(date: string, time: string): string | null {
    if (!date) return null;
    return time ? `${date}T${time}:00` : date;
}

/**
 * When editing a timed value that arrived with an explicit UTC/offset suffix
 * and no named non-UTC timezone, returns the host IANA timezone so saving
 * local wall-clock digits is not re-interpreted as UTC by the backend.
 */
export function localTimezoneIfUtc(
    previousValues: readonly (string | null | undefined)[],
    timezone: string | null | undefined,
): string | undefined {
    if (timezone && !isUtcZone(timezone)) return undefined;
    if (!previousValues.some((value) => hasExplicitOffset(value))) return undefined;
    return Intl.DateTimeFormat().resolvedOptions().timeZone;
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
export function formatFriendlyDate(iso?: string | null, timezone?: string | null): string {
    const date = getFormattedDate(iso, timezone);
    if (!date) return '';
    return friendlyDate.format(parseLocalYmd(date));
}

/** Unified readable task date/time summary such as "Mon, Nov 5 · 14:30". */
export function formatTaskDateTime(
    iso: string | null | undefined,
    placeholder: string,
    timezone?: string | null,
): string {
    const date = formatFriendlyDate(iso, timezone);
    if (!date) return placeholder;
    const time = getFormattedTime(iso, timezone);
    return time ? `${date} · ${time}` : date;
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
