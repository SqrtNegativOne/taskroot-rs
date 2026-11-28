import { describe, expect, it } from 'vitest';
import {
    buildCustomRule,
    describeRule,
    isValidRule,
    parseCustomRule,
    parseRule,
    presetForRule,
    ruleForPreset,
    type CustomRecurrence,
} from './recurrence';

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
        expect(presetForRule('   ')).toBe('never');
    });
});

describe('describeRule', () => {
    it('returns crisp labels for empty and preset rules', () => {
        expect(describeRule(undefined)).toBe('Does not repeat');
        expect(describeRule(null)).toBe('Does not repeat');
        expect(describeRule('')).toBe('Does not repeat');
        expect(describeRule('RRULE:FREQ=DAILY')).toBe('Every day');
        expect(describeRule('RRULE:FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR')).toBe('Every weekday');
        expect(describeRule('RRULE:FREQ=WEEKLY')).toBe('Every week');
        expect(describeRule('RRULE:FREQ=MONTHLY')).toBe('Every month');
        expect(describeRule('RRULE:FREQ=YEARLY')).toBe('Every year');
    });

    it('formats custom intervals, weekdays, counts, until dates, and complex RFC rules via rrule.toText()', () => {
        expect(describeRule('RRULE:FREQ=WEEKLY;INTERVAL=2')).toBe('Every 2 weeks');
        expect(describeRule('RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE')).toBe(
            'Every 2 weeks on Monday, Wednesday',
        );
        expect(describeRule('rrule:freq=weekly; byday=mo,fr')).toBe(
            'Every week on Monday, Friday',
        );
        expect(describeRule('RRULE:FREQ=DAILY;COUNT=5')).toBe('Every day for 5 times');
        expect(describeRule('RRULE:FREQ=WEEKLY;UNTIL=20261231T235959Z')).toBe(
            'Every week until December 31, 2026',
        );
        expect(describeRule('RRULE:FREQ=MONTHLY;BYMONTHDAY=15')).toBe('Every month on the 15th');
        expect(describeRule('RRULE:FREQ=MONTHLY;BYDAY=-1FR')).toBe(
            'Every month on the last Friday',
        );
    });

    it('falls back cleanly without throwing on multi-line, unsupported, or invalid rules', () => {
        expect(describeRule('RRULE:FREQ=DAILY\nRRULE:FREQ=WEEKLY')).toBe('RRULE:FREQ=DAILY');
        expect(describeRule('RRULE:FREQ=SECONDLY;INTERVAL=10')).toBe(
            'RRULE:FREQ=SECONDLY;INTERVAL=10',
        );
        expect(describeRule('RRULE:FREQ=BOGUS')).toBe('RRULE:FREQ=BOGUS');
        expect(describeRule('NOT_AN_RRULE')).toBe('NOT_AN_RRULE');
    });
});

describe('parseRule & isValidRule', () => {
    it('accepts valid single-line RRULE strings regardless of case or spaces', () => {
        expect(isValidRule('RRULE:FREQ=DAILY')).toBe(true);
        expect(isValidRule('rrule:freq=weekly; interval=2; byday=mo,we')).toBe(true);
        expect(isValidRule('RRULE:FREQ=MONTHLY;BYMONTHDAY=15')).toBe(true);
        expect(parseRule('RRULE:FREQ=WEEKLY;COUNT=4')).not.toBeNull();
    });

    it('rejects empty, malformed, multi-line, or out-of-range rules', () => {
        expect(isValidRule('')).toBe(false);
        expect(isValidRule(undefined)).toBe(false);
        expect(isValidRule('FREQ=WEEKLY')).toBe(false);
        expect(isValidRule('RRULE:INTERVAL=2')).toBe(false);
        expect(isValidRule('RRULE:FREQ=BOGUS')).toBe(false);
        expect(isValidRule('RRULE:FREQ=DAILY;INTERVAL=0')).toBe(false);
        expect(isValidRule('RRULE:FREQ=DAILY;COUNT=-3')).toBe(false);
        expect(isValidRule('RRULE:FREQ=WEEKLY;BYDAY=XX')).toBe(false);
        expect(isValidRule('RRULE:FREQ=DAILY\nRRULE:FREQ=WEEKLY')).toBe(false);
    });
});

describe('parseCustomRule & buildCustomRule', () => {
    const cases: { name: string; custom: CustomRecurrence; wire: string }[] = [
        {
            name: 'every day (default interval omitted)',
            custom: {
                freq: 'daily',
                interval: 1,
                byweekday: [],
                endKind: 'never',
                count: 10,
                until: '',
            },
            wire: 'RRULE:FREQ=DAILY',
        },
        {
            name: 'every 2 weeks on Monday and Wednesday',
            custom: {
                freq: 'weekly',
                interval: 2,
                byweekday: ['MO', 'WE'],
                endKind: 'never',
                count: 10,
                until: '',
            },
            wire: 'RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE',
        },
        {
            name: 'every 3 months for 6 times',
            custom: {
                freq: 'monthly',
                interval: 3,
                byweekday: [],
                endKind: 'count',
                count: 6,
                until: '',
            },
            wire: 'RRULE:FREQ=MONTHLY;INTERVAL=3;COUNT=6',
        },
        {
            name: 'every year until a date',
            custom: {
                freq: 'yearly',
                interval: 1,
                byweekday: [],
                endKind: 'until',
                count: 10,
                until: '2028-06-15',
            },
            wire: 'RRULE:FREQ=YEARLY;UNTIL=20280615T235959Z',
        },
    ];

    for (const { name, custom, wire } of cases) {
        it(`round-trips ${name}`, () => {
            const built = buildCustomRule(custom);
            expect(built).toBe(wire);
            expect(built.startsWith('RRULE:')).toBe(true);
            expect(built).not.toContain('DTSTART');
            expect(parseCustomRule(built)).toEqual(custom);
        });
    }

    it('canonicalizes weekday ordering and ignores weekdays on non-weekly frequencies', () => {
        expect(
            buildCustomRule({
                freq: 'weekly',
                interval: 1,
                byweekday: ['FR', 'MO', 'FR'],
                endKind: 'never',
                count: 10,
                until: '',
            }),
        ).toBe('RRULE:FREQ=WEEKLY;BYDAY=MO,FR');

        expect(
            buildCustomRule({
                freq: 'monthly',
                interval: 2,
                byweekday: ['MO', 'WE'],
                endKind: 'never',
                count: 10,
                until: '',
            }),
        ).toBe('RRULE:FREQ=MONTHLY;INTERVAL=2');
    });

    it('returns null from parseCustomRule for advanced RFC rules outside the structured builder', () => {
        expect(parseCustomRule('RRULE:FREQ=MONTHLY;BYMONTHDAY=15')).toBeNull();
        expect(parseCustomRule('RRULE:FREQ=MONTHLY;BYDAY=-1FR')).toBeNull();
        expect(parseCustomRule('RRULE:FREQ=HOURLY;INTERVAL=2')).toBeNull();
        expect(parseCustomRule('RRULE:FREQ=DAILY;COUNT=5;UNTIL=20261231T235959Z')).toBeNull();
    });
});
