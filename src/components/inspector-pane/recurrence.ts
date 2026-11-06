export type RecurrencePreset =
    | 'never'
    | 'daily'
    | 'weekdays'
    | 'weekly'
    | 'monthly'
    | 'yearly'
    | 'custom';

export interface RecurrenceOption {
    value: RecurrencePreset;
    label: string;
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

const WEEKDAYS_RULE = 'RRULE:FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR';

const PRESET_RULES: Record<Exclude<RecurrencePreset, 'custom'>, string | undefined> = {
    never: undefined,
    daily: 'RRULE:FREQ=DAILY',
    weekdays: WEEKDAYS_RULE,
    weekly: 'RRULE:FREQ=WEEKLY',
    monthly: 'RRULE:FREQ=MONTHLY',
    yearly: 'RRULE:FREQ=YEARLY',
};

/** The wire-form rule for a preset, or `undefined` for "does not repeat". */
export function ruleForPreset(preset: RecurrencePreset): string | undefined {
    return preset === 'custom' ? undefined : PRESET_RULES[preset];
}

function ruleLines(rule?: string | null): string[] {
    return (rule ?? '')
        .split('\n')
        .map((part) => part.trim().toUpperCase())
        .filter((part) => part.startsWith('RRULE:'))
        .map((part) => part.replaceAll(/\s+/g, ''));
}

function firstRuleLine(rule?: string | null): string | undefined {
    return ruleLines(rule)[0];
}

/** Which preset (if any) describes an existing rule. */
export function presetForRule(rule?: string | null): RecurrencePreset {
    const lines = ruleLines(rule);
    if (lines.length > 1) return 'custom';
    const line = lines[0];
    if (line === undefined) return 'never';
    if (line === WEEKDAYS_RULE) return 'weekdays';

    const entry = (Object.entries(PRESET_RULES) as [RecurrencePreset, string | undefined][]).find(
        ([preset, value]) => preset !== 'custom' && preset !== 'never' && value === line,
    );
    return entry?.[0] ?? 'custom';
}

/** A short human summary of a rule, used where the rule itself is not shown. */
export function describeRule(rule?: string | null): string {
    const preset = presetForRule(rule);
    if (preset === 'never') return 'Does not repeat';
    if (preset !== 'custom') {
        return RECURRENCE_OPTIONS.find((option) => option.value === preset)?.label ?? 'Custom';
    }
    return firstRuleLine(rule) ?? 'Custom';
}
