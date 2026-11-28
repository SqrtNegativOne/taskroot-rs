export type Rgb = readonly [number, number, number];

export type EventColorStyle = 'tint' | 'solid';

export interface EventTheme {
    readonly surface: Rgb;
    readonly onSurface: Rgb;
    readonly muted: Rgb;
    readonly style: EventColorStyle;
}

export interface EventColors {
    readonly fill: Rgb;
    readonly text: Rgb;
    readonly secondary: Rgb;
    readonly bar: Rgb;
}

export interface EventColorSource {
    readonly color?: string | null;
    /** `EventInstance` calendar reference. */
    readonly calendarId?: string | null;
    /** `AppEvent` calendar reference. */
    readonly remoteCollectionId?: string | null;
}

export interface CalendarColorSource {
    readonly id: string;
    readonly color?: string | null;
}

const DEFAULT_CALENDAR_COLOR: Rgb = [138, 180, 248];
const WHITE: Rgb = [255, 255, 255];
const BLACK: Rgb = [0, 0, 0];
const MIN_CONTRAST = 4.5;
const TINT_MAX = 0.18;
const TINT_MIN = 0.02;
const TINT_STEP = 0.02;

function channelLuminance(value: number): number {
    const channel = value / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
}

function relativeLuminance([red, green, blue]: Rgb): number {
    return 0.2126 * channelLuminance(red)
        + 0.7152 * channelLuminance(green)
        + 0.0722 * channelLuminance(blue);
}

export function contrastRatio(first: Rgb, second: Rgb): number {
    const a = relativeLuminance(first);
    const b = relativeLuminance(second);
    const high = Math.max(a, b);
    const low = Math.min(a, b);
    return (high + 0.05) / (low + 0.05);
}

function mixColor(from: Rgb, to: Rgb, shareOfTo: number): Rgb {
    return [
        Math.round(from[0] * (1 - shareOfTo) + to[0] * shareOfTo),
        Math.round(from[1] * (1 - shareOfTo) + to[1] * shareOfTo),
        Math.round(from[2] * (1 - shareOfTo) + to[2] * shareOfTo),
    ];
}

/**
 * Parses a CalDAV colour, accepting `#RRGGBB` and `#RRGGBBAA`. The alpha byte
 * is dropped so the opaque calendar colour drives the derived fill and text.
 */
export function parseColor(raw?: string): Rgb | undefined {
    if (!raw) return undefined;
    const match = /^#([0-9a-f]{6})(?:[0-9a-f]{2})?$/i.exec(raw.trim());
    if (!match) return undefined;
    const hex = match[1];
    return [
        Number.parseInt(hex.slice(0, 2), 16),
        Number.parseInt(hex.slice(2, 4), 16),
        Number.parseInt(hex.slice(4, 6), 16),
    ];
}

function tintFill(surface: Rgb, calendar: Rgb, onSurface: Rgb): Rgb {
    const first = Math.round(TINT_MAX / TINT_STEP);
    const last = Math.round(TINT_MIN / TINT_STEP);
    for (let step = first; step >= last; step -= 1) {
        const fill = mixColor(surface, calendar, step * TINT_STEP);
        if (contrastRatio(fill, onSurface) >= MIN_CONTRAST) return fill;
    }
    return mixColor(surface, calendar, TINT_MIN);
}

function secondaryText(fill: Rgb, muted: Rgb, onSurface: Rgb): Rgb {
    return contrastRatio(muted, fill) >= MIN_CONTRAST ? muted : onSurface;
}

/**
 * Derives the fill, text and accent bar for one calendar colour. Tint keeps the
 * theme's surface and text roles; solid picks black or white by contrast.
 */
export function eventColors(calendar: Rgb, theme: EventTheme): EventColors {
    if (theme.style === 'solid') {
        const text = contrastRatio(calendar, WHITE) >= contrastRatio(calendar, BLACK)
            ? WHITE
            : BLACK;
        const dimmed = mixColor(text, calendar, 0.2);
        const secondary = contrastRatio(dimmed, calendar) >= MIN_CONTRAST ? dimmed : text;
        return { fill: calendar, text, secondary, bar: calendar };
    }

    const fill = tintFill(theme.surface, calendar, theme.onSurface);
    return {
        fill,
        text: theme.onSurface,
        secondary: secondaryText(fill, theme.muted, theme.onSurface),
        bar: calendar,
    };
}

function cssRgb([red, green, blue]: Rgb): string {
    return `rgb(${red} ${green} ${blue})`;
}

/**
 * Inline CSS custom properties shared by every event render surface. An absent
 * colour leaves the variables unset so each surface keeps its own default; an
 * unparseable colour falls back to the default calendar palette.
 */
export function eventColorCss(color: string | undefined, theme: EventTheme): string {
    if (!color) return '';
    const calendar = parseColor(color) ?? DEFAULT_CALENDAR_COLOR;
    const colors = eventColors(calendar, theme);
    return [
        `--ev-color: ${cssRgb(colors.bar)}`,
        `--ev-bg: ${cssRgb(colors.fill)}`,
        `--ev-text: ${cssRgb(colors.text)}`,
        `--ev-secondary: ${cssRgb(colors.secondary)}`,
    ].join('; ');
}

/**
 * The colour an event renders with: its own `COLOR` override, otherwise the
 * colour of the calendar it belongs to. `undefined` means "use the surface
 * default".
 */
export function resolveEventColor(
    event: EventColorSource,
    calendars: readonly CalendarColorSource[] = [],
): string | undefined {
    if (event.color) return event.color;
    const calendarId = event.calendarId ?? event.remoteCollectionId;
    if (!calendarId) return undefined;
    return calendars.find((calendar) => calendar.id === calendarId)?.color ?? undefined;
}
