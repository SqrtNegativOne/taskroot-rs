import { describe, expect, it } from 'vitest';
import { eventColorVars, withAlpha } from './eventColor';

describe('withAlpha', () => {
    it('adds an alpha suffix to a six-digit hex color', () => {
        const translucent = withAlpha('#4285f4');
        expect(translucent).toBe('#4285f440');
        expect(translucent.startsWith('#4285f4')).toBe(true);
    });

    it('leaves colors that already carry alpha untouched', () => {
        expect(withAlpha('#4285f480')).toBe('#4285f480');
    });
});

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
});
