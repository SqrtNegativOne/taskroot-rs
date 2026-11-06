import { parseLocalYmd } from '../../lib/time';

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

export function getFormattedDate(iso?: string): string {
    if (!iso) return '';
    return iso.split('T')[0] ?? '';
}

export function getFormattedTime(iso?: string): string {
    if (!iso?.includes('T')) return '';
    const t = iso.split('T')[1];
    return t?.substring(0, 5) ?? '';
}

/** Replace the date of a canonical value, keeping its time (if any). */
export function withDate(iso: string | undefined, date: string): string | undefined {
    if (!date) return undefined;
    const time = iso?.includes('T') ? iso.slice(iso.indexOf('T')) : '';
    return `${date}${time}`;
}

/** Replace the time of a canonical value; an empty time makes it date-only. */
export function withTime(iso: string | undefined, time: string): string | undefined {
    const date = iso?.split('T')[0];
    if (!date) return undefined;
    return time ? `${date}T${time}:00` : date;
}

/** A short, human date such as "Mon, Nov 5". */
export function formatFriendlyDate(iso?: string): string {
    const date = getFormattedDate(iso);
    if (!date) return '';
    return friendlyDate.format(parseLocalYmd(date));
}

/** A human date with its year, for read-only metadata. */
export function formatFriendlyDateWithYear(iso?: string): string {
    const date = getFormattedDate(iso);
    if (!date) return '';
    return friendlyDateWithYear.format(parseLocalYmd(date));
}

/** A read-only timestamp (created/modified) in local time. */
export function formatTimestamp(iso?: string): string {
    if (!iso) return '';
    const date = new Date(iso);
    return Number.isNaN(date.getTime()) ? iso : friendlyTimestamp.format(date);
}
