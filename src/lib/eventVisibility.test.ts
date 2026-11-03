import { describe, expect, it } from 'vitest';
import { makeInstance } from '../test/factories';
import { isEventVisible, isEventVisibility } from './eventVisibility';

const events = [
    makeInstance({ id: 'one-off-all-day', timing: { kind: 'allDay' } }),
    makeInstance({ id: 'one-off-timed' }),
    makeInstance({ id: 'recurring-all-day', recurring: true, timing: { kind: 'allDay' } }),
    makeInstance({ id: 'recurring-timed', recurring: true }),
    makeInstance({ id: 'moved-exception', recurring: true }),
];

describe('event visibility', () => {
    it.each([
        { hideRecurring: false, hideTimed: false, ids: ['one-off-all-day', 'one-off-timed', 'recurring-all-day', 'recurring-timed', 'moved-exception'] },
        { hideRecurring: true, hideTimed: false, ids: ['one-off-all-day', 'one-off-timed'] },
        { hideRecurring: false, hideTimed: true, ids: ['one-off-all-day', 'recurring-all-day'] },
        { hideRecurring: true, hideTimed: true, ids: ['one-off-all-day'] },
    ])('applies independent visibility choices: recurring=$hideRecurring, timed=$hideTimed', ({ hideRecurring, hideTimed, ids }) => {
        expect(events.filter(event => isEventVisible(event, { hideRecurring, hideTimed })).map(event => event.id)).toEqual(ids);
    });

    it('accepts only stored visibility choices with both boolean fields', () => {
        expect(isEventVisibility({ hideRecurring: true, hideTimed: false })).toBe(true);
        for (const value of [null, [], false, {}, { hideRecurring: true }, { hideRecurring: 'false', hideTimed: false }]) {
            expect(isEventVisibility(value)).toBe(false);
        }
    });
});
