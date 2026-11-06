import { describe, expect, it } from 'vitest';
import {
    describeRule,
    presetForRule,
    ruleForPreset,
    RECURRENCE_OPTIONS,
} from './recurrence';

describe('ruleForPreset', () => {
    it('builds the wire form for each concrete preset', () => {
        expect(ruleForPreset('daily')).toBe('RRULE:FREQ=DAILY');
        expect(ruleForPreset('weekly')).toBe('RRULE:FREQ=WEEKLY');
        expect(ruleForPreset('weekdays')).toBe('RRULE:FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR');
    });

    it('has no rule for "never" or "custom"', () => {
        expect(ruleForPreset('never')).toBeUndefined();
        expect(ruleForPreset('custom')).toBeUndefined();
    });
});

describe('presetForRule', () => {
    it('recognises the presets it writes', () => {
        for (const preset of ['daily', 'weekdays', 'weekly', 'monthly', 'yearly'] as const) {
            expect(presetForRule(ruleForPreset(preset))).toBe(preset);
        }
    });

    it('is case- and whitespace-insensitive', () => {
        expect(presetForRule('rrule:freq=weekly; byday=mo,tu,we,th,fr')).toBe('weekdays');
    });

    it('treats anything else as custom', () => {
        expect(presetForRule('RRULE:FREQ=WEEKLY;INTERVAL=2')).toBe('custom');
        expect(presetForRule('RRULE:FREQ=DAILY\nRRULE:FREQ=WEEKLY')).toBe('custom');
    });

    it('treats a missing rule as never', () => {
        expect(presetForRule(undefined)).toBe('never');
        expect(presetForRule('')).toBe('never');
    });
});

describe('describeRule', () => {
    it('names a known preset', () => {
        expect(describeRule('RRULE:FREQ=MONTHLY')).toBe('Every month');
    });

    it('falls back to the raw rule for custom schedules', () => {
        expect(describeRule('RRULE:FREQ=WEEKLY;INTERVAL=2')).toBe('RRULE:FREQ=WEEKLY;INTERVAL=2');
    });

    it('describes a missing rule as not repeating', () => {
        expect(describeRule(undefined)).toBe('Does not repeat');
    });
});

describe('options', () => {
    it('offers every preset exactly once', () => {
        const values = RECURRENCE_OPTIONS.map((option) => option.value);
        expect(new Set(values).size).toBe(values.length);
        expect(values).toContain('custom');
    });
});
