const MS_PER_DAY = 86_400_000;

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
