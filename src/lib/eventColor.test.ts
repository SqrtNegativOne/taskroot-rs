import { describe, expect, it } from 'vitest';
import { eventColorVars, resolveEventColor } from './eventColor';

describe('eventColorVars', () => {
    it('produces no variables when the event has no color', () => {
        expect(eventColorVars(undefined)).toBe('');
        expect(eventColorVars('')).toBe('');
    });

    it('exposes the event color as a border accent and a translucent surface', () => {
        const vars = eventColorVars('#4285f4');
        expect(vars).toContain('--ev-color: #4285f4');
        expect(vars).toContain('--ev-bg: #4285f440');
    });

    it('leaves colors that already carry alpha untouched', () => {
        expect(eventColorVars('#4285f480')).toContain('--ev-bg: #4285f480');
    });
});

describe('resolveEventColor', () => {
    const calendars = [
        { id: 'cal-events', color: '#123456' },
        { id: 'cal-empty' },
    ];

    it('prefers the event override over the calendar colour', () => {
        expect(
            resolveEventColor({ color: '#abcdef', calendarId: 'cal-events' }, calendars),
        ).toBe('#abcdef');
    });

    it('falls back to the calendar colour for an event without an override', () => {
        expect(resolveEventColor({ calendarId: 'cal-events' }, calendars)).toBe('#123456');
    });

    it('resolves a raw AppEvent through remoteCollectionId', () => {
        expect(
            resolveEventColor({ remoteCollectionId: 'cal-events' }, calendars),
        ).toBe('#123456');
    });

    it('returns undefined when the calendar has no colour or is unknown', () => {
        expect(resolveEventColor({ calendarId: 'cal-empty' }, calendars)).toBeUndefined();
        expect(resolveEventColor({ calendarId: 'missing' }, calendars)).toBeUndefined();
        expect(resolveEventColor({}, calendars)).toBeUndefined();
    });
});
