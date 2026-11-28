import { Frequency, RRule, Weekday, datetime, type Options, type WeekdayStr } from 'rrule';

export type RecurrencePreset =
    | 'never' | 'daily' | 'weekdays' | 'weekly' | 'monthly' | 'yearly' | 'custom';
export type CustomFrequency = 'daily' | 'weekly' | 'monthly' | 'yearly';
export type RecurrenceWeekday = WeekdayStr;
export type RecurrenceEndKind = 'never' | 'count' | 'until';

export interface RecurrenceOption {
    value: RecurrencePreset;
    label: string;
}

export interface CustomRecurrence {
    freq: CustomFrequency;
    interval: number;
    byweekday: RecurrenceWeekday[];
    endKind: RecurrenceEndKind;
    count: number;
    until: string;
}

export const RECURRENCE_OPTIONS: readonly RecurrenceOption[] = [
    { value: 'never', label: 'Does not repeat' },
    { value: 'daily', label: 'Every day' },
    { value: 'weekdays', label: 'Every weekday' },
    { value: 'weekly', label: 'Every week' },
    { value: 'monthly', label: 'Every month' },
    { value: 'yearly', label: 'Every year' },
    { value: 'custom', label: 'Custom' },
];

export const WEEKDAY_ORDER: readonly RecurrenceWeekday[] = ['MO', 'TU', 'WE', 'TH', 'FR', 'SA', 'SU'];

export const DEFAULT_CUSTOM_RECURRENCE: CustomRecurrence = {
    freq: 'weekly', interval: 1, byweekday: [], endKind: 'never', count: 10, until: '',
};

const WEEKDAYS_RULE = 'RRULE:FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR';
const YMD_PATTERN = /^(\d{4})-(\d{2})-(\d{2})$/;

const PRESET_RULES: Record<Exclude<RecurrencePreset, 'custom'>, string | undefined> = {
    never: undefined, daily: 'RRULE:FREQ=DAILY', weekdays: WEEKDAYS_RULE,
    weekly: 'RRULE:FREQ=WEEKLY', monthly: 'RRULE:FREQ=MONTHLY', yearly: 'RRULE:FREQ=YEARLY',
};

const FREQ_TO_RRULE: Record<CustomFrequency, Frequency> = {
    daily: Frequency.DAILY, weekly: Frequency.WEEKLY, monthly: Frequency.MONTHLY, yearly: Frequency.YEARLY,
};

const RRULE_TO_FREQ: Partial<Record<Frequency, CustomFrequency>> = {
    [Frequency.DAILY]: 'daily', [Frequency.WEEKLY]: 'weekly',
    [Frequency.MONTHLY]: 'monthly', [Frequency.YEARLY]: 'yearly',
};

const VALID_FREQS = new Set(['YEARLY', 'MONTHLY', 'WEEKLY', 'DAILY', 'HOURLY', 'MINUTELY', 'SECONDLY']);
const ADVANCED_KEYS: readonly (keyof Options)[] = [
    'bymonth', 'bymonthday', 'bynmonthday', 'byyearday', 'byweekno', 'bysetpos',
    'byhour', 'byminute', 'bysecond', 'byeaster', 'dtstart', 'tzid', 'wkst',
];

/** The wire-form rule for a preset, or `undefined` for "does not repeat". */
export function ruleForPreset(preset: RecurrencePreset): string | undefined {
    return preset === 'custom' ? undefined : PRESET_RULES[preset];
}

function nonEmptyLines(rule?: string | null): string[] {
    return (rule ?? '').split('\n').map((part) => part.trim()).filter((part) => part.length > 0);
}

function ruleLines(rule?: string | null): string[] {
    return nonEmptyLines(rule)
        .map((part) => part.toUpperCase().replaceAll(/\s+/g, ''))
        .filter((part) => part.startsWith('RRULE:'));
}

function isPositiveInt(value: unknown): value is number {
    return typeof value === 'number' && Number.isInteger(value) && value >= 1;
}

function formatUtcYmd(date: Date): string {
    const y = date.getUTCFullYear().toString().padStart(4, '0');
    const m = (date.getUTCMonth() + 1).toString().padStart(2, '0');
    const d = date.getUTCDate().toString().padStart(2, '0');
    return `${y}-${m}-${d}`;
}

function parseYmdUtcEnd(value: string): Date | null {
    const match = YMD_PATTERN.exec(value);
    if (!match) return null;
    const [year, month, day] = [Number(match[1]), Number(match[2]), Number(match[3])];
    const date = datetime(year, month, day, 23, 59, 59);
    const same = date.getUTCFullYear() === year && date.getUTCMonth() + 1 === month && date.getUTCDate() === day;
    return same ? date : null;
}

function hasValidKeyValues(body: string): boolean {
    if (!body) return false;
    let hasFreq = false;
    for (const part of body.split(';')) {
        const [key, val, extra] = part.split('=');
        if (!key || !val || extra !== undefined) return false;
        if (key === 'FREQ') {
            if (!VALID_FREQS.has(val)) return false;
            hasFreq = true;
        }
    }
    return hasFreq;
}

function hasValidOptions(options: Partial<Options>): boolean {
    if (options.freq === undefined) return false;
    if (options.interval !== undefined && !isPositiveInt(options.interval)) return false;
    if (options.count != null && !isPositiveInt(options.count)) return false;
    if (options.until != null && (!(options.until instanceof Date) || Number.isNaN(options.until.getTime()))) {
        return false;
    }
    if (options.byweekday != null) {
        const items = Array.isArray(options.byweekday) ? options.byweekday : [options.byweekday];
        if (items.length === 0 || items.some((day) => !(day instanceof Weekday))) return false;
    }
    return true;
}

/** Parse a single RFC 5545 `RRULE:...` line into an `RRule`, or `null` if invalid or multi-line. */
export function parseRule(rule?: string | null): RRule | null {
    const lines = nonEmptyLines(rule);
    if (lines.length !== 1) return null;

    const normalized = lines[0].toUpperCase().replaceAll(/\s+/g, '');
    if (!normalized.startsWith('RRULE:') || !hasValidKeyValues(normalized.slice('RRULE:'.length))) {
        return null;
    }

    try {
        const options = RRule.parseString(normalized);
        return hasValidOptions(options) ? new RRule(options) : null;
    } catch {
        return null;
    }
}

/** Whether a raw string is a valid single-line `RRULE:...`. */
export function isValidRule(rule?: string | null): boolean {
    return parseRule(rule) !== null;
}

/** Which preset (if any) describes an existing rule. */
export function presetForRule(rule?: string | null): RecurrencePreset {
    const allLines = nonEmptyLines(rule);
    if (allLines.length === 0) return 'never';

    const lines = ruleLines(rule);
    if (lines.length !== 1 || allLines.length > 1) return 'custom';
    const line = lines[0];
    if (line === WEEKDAYS_RULE) return 'weekdays';

    const entry = (Object.entries(PRESET_RULES) as [RecurrencePreset, string | undefined][]).find(
        ([preset, value]) => preset !== 'custom' && preset !== 'never' && value === line,
    );
    return entry?.[0] ?? 'custom';
}

/** A human-readable summary of a rule powered by `rrule.toText()`, falling back cleanly on error. */
export function describeRule(rule?: string | null): string {
    const trimmed = (rule ?? '').trim();
    if (!trimmed) return 'Does not repeat';

    const preset = presetForRule(trimmed);
    if (preset !== 'custom' && preset !== 'never') {
        return RECURRENCE_OPTIONS.find((option) => option.value === preset)?.label ?? 'Custom';
    }

    const fallback = ruleLines(trimmed)[0] ?? trimmed;
    const parsed = parseRule(trimmed);
    if (!parsed) return fallback;

    try {
        const text = parsed.toText().trim();
        if (!text || text.startsWith('RRule error:')) return fallback;
        return text.charAt(0).toUpperCase() + text.slice(1);
    } catch {
        return fallback;
    }
}

function extractPlainWeekdays(byweekday: Options['byweekday']): RecurrenceWeekday[] | null {
    if (byweekday == null) return [];
    const items = Array.isArray(byweekday) ? byweekday : [byweekday];
    const picked = new Set<RecurrenceWeekday>();
    for (const item of items) {
        if (!(item instanceof Weekday) || item.n !== undefined) return null;
        const code = WEEKDAY_ORDER[item.weekday];
        if (!code) return null;
        picked.add(code);
    }
    return WEEKDAY_ORDER.filter((day) => picked.has(day));
}

function extractEnd(orig: Partial<Options>): Pick<CustomRecurrence, 'endKind' | 'count' | 'until'> {
    if (orig.count != null) return { endKind: 'count', count: orig.count, until: '' };
    if (orig.until instanceof Date) {
        return { endKind: 'until', count: DEFAULT_CUSTOM_RECURRENCE.count, until: formatUtcYmd(orig.until) };
    }
    return { endKind: 'never', count: DEFAULT_CUSTOM_RECURRENCE.count, until: '' };
}

/** Parse a rule into structured custom-builder fields, or `null` if outside the structured builder. */
export function parseCustomRule(rule?: string | null): CustomRecurrence | null {
    const parsed = parseRule(rule);
    if (!parsed) return null;

    const orig = parsed.origOptions;
    const freq = orig.freq !== undefined ? RRULE_TO_FREQ[orig.freq] : undefined;
    const hasConflict = orig.count != null && orig.until != null;
    if (!freq || hasConflict || ADVANCED_KEYS.some((key) => orig[key] != null)) return null;

    const byweekday = extractPlainWeekdays(orig.byweekday ?? null);
    if (byweekday === null || (byweekday.length > 0 && freq !== 'weekly')) return null;

    const interval = isPositiveInt(orig.interval) ? orig.interval : 1;
    return { freq, interval, byweekday, ...extractEnd(orig) };
}

/** Serialize structured custom-builder fields into a canonical wire-form `RRULE:...` string. */
export function buildCustomRule(custom: CustomRecurrence): string {
    const interval = Math.max(1, Math.floor(custom.interval) || 1);
    const options: Partial<Options> = { freq: FREQ_TO_RRULE[custom.freq] };

    if (interval > 1) options.interval = interval;
    if (custom.freq === 'weekly' && custom.byweekday.length > 0) {
        const picked = new Set(custom.byweekday);
        const ordered = WEEKDAY_ORDER.filter((day) => picked.has(day));
        if (ordered.length > 0) options.byweekday = ordered.map((day) => Weekday.fromStr(day));
    }
    if (custom.endKind === 'count') {
        options.count = Math.max(1, Math.floor(custom.count) || 1);
    } else if (custom.endKind === 'until') {
        const until = parseYmdUtcEnd(custom.until);
        if (until) options.until = until;
    }

    return new RRule(options).toString();
}
