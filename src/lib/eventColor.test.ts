import { describe, expect, it } from 'vitest';
import {
    contrastRatio,
    eventColorCss,
    eventColors,
    parseColor,
    resolveEventColor,
    type EventTheme,
} from './eventColor';

const TINT_THEME: EventTheme = {
    surface: [34, 43, 56],
    onSurface: [227, 234, 244],
    muted: [169, 184, 204],
    style: 'tint',
};

const SOLID_THEME: EventTheme = { ...TINT_THEME, style: 'solid' };

const CALENDAR_COLORS = [
    [160, 50, 96],
    [80, 160, 220],
    [230, 210, 120],
    [40, 40, 40],
    [250, 250, 250],
] as const;

describe('calendar colour parsing', () => {
    it('drops the CalDAV alpha byte', () => {
        expect(parseColor('#a03260ff')).toEqual([160, 50, 96]);
        expect(parseColor('#a0326080')).toEqual([160, 50, 96]);
    });

    it('accepts opaque colours and rejects malformed values', () => {
        expect(parseColor('#A03260')).toEqual([160, 50, 96]);
        expect(parseColor('a03260')).toBeUndefined();
        expect(parseColor('#a0326')).toBeUndefined();
        expect(parseColor('#gg2260')).toBeUndefined();
        expect(parseColor('')).toBeUndefined();
    });
});

describe('event colour derivation', () => {
    it('uses the theme surface and text for the default tint style', () => {
        for (const calendar of CALENDAR_COLORS) {
            const colors = eventColors(calendar, TINT_THEME);
            expect(colors.bar).toEqual(calendar);
            expect(colors.text).toEqual(TINT_THEME.onSurface);
            expect(colors.fill).not.toEqual(calendar);
            expect(contrastRatio(colors.fill, colors.text)).toBeGreaterThanOrEqual(4.5);
            expect(contrastRatio(colors.secondary, colors.fill)).toBeGreaterThanOrEqual(4.5);
        }
    });

    it('keeps tinted fills close to the surface rather than replacing it', () => {
        const colors = eventColors([250, 250, 250], TINT_THEME);
        expect(contrastRatio(colors.fill, TINT_THEME.surface)).toBeLessThan(4.5);
        expect(contrastRatio(colors.fill, TINT_THEME.onSurface)).toBeGreaterThanOrEqual(4.5);
    });

    it('picks black or white text by contrast in the solid style', () => {
        expect(eventColors([30, 20, 60], SOLID_THEME).text).toEqual([255, 255, 255]);
        expect(eventColors([240, 230, 200], SOLID_THEME).text).toEqual([0, 0, 0]);
    });

    it('keeps solid fills and their secondary text readable', () => {
        for (const calendar of CALENDAR_COLORS) {
            const colors = eventColors(calendar, SOLID_THEME);
            expect(colors.fill).toEqual(calendar);
            expect(colors.bar).toEqual(calendar);
            expect(contrastRatio(colors.text, colors.fill)).toBeGreaterThanOrEqual(4.5);
            expect(contrastRatio(colors.secondary, colors.fill)).toBeGreaterThanOrEqual(4.5);
        }
    });
});

describe('event colour CSS variables', () => {
    it('leaves surfaces on their defaults when no colour is stored', () => {
        expect(eventColorCss(undefined, TINT_THEME)).toBe('');
    });

    it('publishes the derived fill, text and bar for a stored colour', () => {
        const css = eventColorCss('#a03260ff', TINT_THEME);
        expect(css).toContain('--ev-color: rgb(160 50 96)');
        expect(css).toContain('--ev-bg: rgb(');
        expect(css).toContain('--ev-text: rgb(227 234 244)');
        expect(css).toContain('--ev-secondary: rgb(');
        expect(css).not.toContain('--ev-bg: rgb(160 50 96)');
    });

    it('falls back to the default palette for malformed colours', () => {
        const css = eventColorCss('not-a-colour', TINT_THEME);
        expect(css).toContain('--ev-color: rgb(138 180 248)');
        expect(css).toContain('--ev-bg: rgb(');
    });
});

describe('event colour resolution', () => {
    const calendars = [{ id: 'work', color: '#123456' }];

    it('prefers the event colour over its calendar', () => {
        expect(resolveEventColor({ color: '#abcdef', calendarId: 'work' }, calendars))
            .toBe('#abcdef');
    });

    it('falls back to the calendar colour', () => {
        expect(resolveEventColor({ calendarId: 'work' }, calendars)).toBe('#123456');
        expect(resolveEventColor({ remoteCollectionId: 'work' }, calendars)).toBe('#123456');
    });

    it('returns undefined when neither source has a colour', () => {
        expect(resolveEventColor({ calendarId: 'missing' }, calendars)).toBeUndefined();
        expect(resolveEventColor({}, calendars)).toBeUndefined();
    });
});
