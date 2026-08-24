export function getFormattedDate(iso?: string): string {
    if (!iso) return '';
    return iso.split('T')[0];
}

export function getFormattedTime(iso?: string): string {
    if (!iso?.includes('T')) return '';
    const t = iso.split('T')[1];
    return t.substring(0, 5);
}
