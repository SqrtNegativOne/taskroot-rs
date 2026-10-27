export function getFormattedDate(iso?: string): string {
    if (!iso) return '';
    return iso.split('T')[0];
}

export function getFormattedTime(iso?: string): string {
    if (!iso?.includes('T')) return '';
    const t = iso.split('T')[1];
    return t.substring(0, 5);
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
