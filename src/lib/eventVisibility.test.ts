import { describe, expect, it } from 'vitest';
import { makeInstance } from '../test/factories';
import { isEventVisible, isEventVisibility, isFrequentRecurrence } from './eventVisibility';

const events = [
    makeInstance({ id: 'one-off-all-day', timing: { kind: 'allDay' } }),
    makeInstance({ id: 'one-off-timed' }),
    makeInstance({ id: 'daily-all-day', recurring: true, rrule: 'RRULE:FREQ=DAILY', timing: { kind: 'allDay' } }),
    makeInstance({ id: 'every-four-days', recurring: true, rrule: 'RRULE:FREQ=DAILY;INTERVAL=4' }),
    makeInstance({ id: 'weekly-timed', recurring: true, rrule: 'RRULE:FREQ=WEEKLY;BYDAY=MO' }),
    makeInstance({ id: 'monthly-timed', recurring: true, rrule: 'RRULE:FREQ=MONTHLY' }),
    makeInstance({ id: 'yearly-timed', recurring: true, rrule: 'RRULE:FREQ=YEARLY' }),
];

describe('event visibility', () => {
    it.each([
        { hideFrequentRecurring: false, hideTimed: false, ids: ['one-off-all-day', 'one-off-timed', 'daily-all-day', 'every-four-days', 'weekly-timed', 'monthly-timed', 'yearly-timed'] },
        { hideFrequentRecurring: true, hideTimed: false, ids: ['one-off-all-day', 'one-off-timed', 'monthly-timed', 'yearly-timed'] },
        { hideFrequentRecurring: false, hideTimed: true, ids: ['one-off-all-day', 'daily-all-day'] },
        { hideFrequentRecurring: true, hideTimed: true, ids: ['one-off-all-day'] },
    ])('applies independent visibility choices: frequent=$hideFrequentRecurring, timed=$hideTimed', ({ hideFrequentRecurring, hideTimed, ids }) => {
        expect(events.filter(event => isEventVisible(event, { hideFrequentRecurring, hideTimed })).map(event => event.id)).toEqual(ids);
    });

    it.each([
        { rrule: undefined, frequent: false },
        { rrule: 'RRULE:FREQ=SECONDLY', frequent: true },
        { rrule: 'RRULE:FREQ=HOURLY', frequent: true },
        { rrule: 'RRULE:FREQ=DAILY;INTERVAL=30', frequent: true },
        { rrule: 'RRULE:FREQ=WEEKLY', frequent: true },
        { rrule: 'RRULE:FREQ=MONTHLY', frequent: false },
        { rrule: 'RRULE:FREQ=YEARLY', frequent: false },
        { rrule: 'rrule:freq=weekly; byday=mo', frequent: true },
    ])('classifies $rrule as frequent=$frequent', ({ rrule, frequent }) => {
        expect(isFrequentRecurrence(makeInstance({ recurring: rrule !== undefined, rrule }))).toBe(frequent);
    });

    it('accepts only stored visibility choices with both boolean fields', () => {
        expect(isEventVisibility({ hideFrequentRecurring: true, hideTimed: false })).toBe(true);
        for (const value of [null, [], false, {}, { hideFrequentRecurring: true }, { hideFrequentRecurring: 'false', hideTimed: false }, { hideRecurring: true, hideTimed: false }]) {
            expect(isEventVisibility(value)).toBe(false);
        }
    });
});
