import { describe, expect, it } from 'vitest';
import { eventColorVars } from './eventColor';

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
