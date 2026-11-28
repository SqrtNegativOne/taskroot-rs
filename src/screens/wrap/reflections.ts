export interface WrapReflectionEntry {
    mood?: number;
    energy?: number;
    wentWell: string;
    improve: string;
}

export type WrapReflectionMap = Record<string, WrapReflectionEntry>;

export const EMPTY_REFLECTION: WrapReflectionEntry = {
    wentWell: '',
    improve: '',
};

export const MOOD_OPTIONS: readonly { value: number; label: string }[] = [
    { value: 1, label: 'Low' },
    { value: 2, label: 'Meh' },
    { value: 3, label: 'Okay' },
    { value: 4, label: 'Good' },
    { value: 5, label: 'Great' },
];

export const ENERGY_OPTIONS: readonly { value: number; label: string }[] = [
    { value: 1, label: 'Drained' },
    { value: 2, label: 'Tired' },
    { value: 3, label: 'Steady' },
    { value: 4, label: 'Focused' },
    { value: 5, label: 'High' },
];

const YMD_KEY_RE = /^\d{4}-\d{2}-\d{2}$/;

function isRating(value: unknown): boolean {
    return value === undefined || (typeof value === 'number' && Number.isInteger(value) && value >= 1 && value <= 5);
}

function isPlainRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isValidEntry(value: unknown): value is WrapReflectionEntry {
    if (!isPlainRecord(value)) return false;
    return (
        typeof value.wentWell === 'string'
        && typeof value.improve === 'string'
        && isRating(value.mood)
        && isRating(value.energy)
    );
}

export function isValidWrapReflections(value: unknown): value is WrapReflectionMap {
    if (!isPlainRecord(value)) return false;
    return Object.entries(value).every(
        ([key, entry]) => YMD_KEY_RE.test(key) && isValidEntry(entry),
    );
}
