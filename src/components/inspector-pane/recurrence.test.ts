import { describe, expect, it } from 'vitest';
import { describeRule, presetForRule, ruleForPreset } from './recurrence';

describe('presetForRule', () => {
    it('round-trips every concrete preset', () => {
        for (const preset of ['daily', 'weekdays', 'weekly', 'monthly', 'yearly'] as const) {
            expect(presetForRule(ruleForPreset(preset))).toBe(preset);
        }
    });

    it('normalizes case and whitespace', () => {
        expect(presetForRule('rrule:freq=weekly; byday=mo,tu,we,th,fr')).toBe('weekdays');
    });

    it('classifies unrecognised or multi-line rules as custom and missing rules as never', () => {
        expect(presetForRule('RRULE:FREQ=WEEKLY;INTERVAL=2')).toBe('custom');
        expect(presetForRule('RRULE:FREQ=DAILY\nRRULE:FREQ=WEEKLY')).toBe('custom');
        expect(presetForRule(undefined)).toBe('never');
        expect(presetForRule('')).toBe('never');
    });

    it('falls back to the raw rule text for custom schedules', () => {
        expect(describeRule('RRULE:FREQ=WEEKLY;INTERVAL=2')).toBe('RRULE:FREQ=WEEKLY;INTERVAL=2');
    });
});
