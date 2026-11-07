const HEX_RGB_PATTERN = /^#[0-9a-fA-F]{6}$/;
const EVENT_BACKGROUND_ALPHA = '40';

export interface EventColorSource {
    readonly color?: string;
    /** `EventInstance` calendar reference. */
    readonly calendarId?: string;
    /** `AppEvent` calendar reference. */
    readonly remoteCollectionId?: string;
}

export interface CalendarColorSource {
    readonly id: string;
    readonly color?: string;
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
    return calendars.find((calendar) => calendar.id === calendarId)?.color;
}

/**
 * Append an 8-bit alpha suffix to a 6-digit hex color. Colors that are already
 * in another notation (or carry their own alpha) are returned unchanged.
 */
function withAlpha(color: string, alphaHex: string = EVENT_BACKGROUND_ALPHA): string {
    return HEX_RGB_PATTERN.test(color) ? `${color}${alphaHex}` : color;
}

/**
 * Inline CSS custom properties shared by every event render surface so the
 * date grid and day timeline cannot drift apart. Empty when the event has no
 * color, letting each surface fall back to its own default.
 */
export function eventColorVars(color?: string): string {
    if (!color) return '';
    return `--ev-color: ${color}; --ev-bg: ${withAlpha(color)};`;
}
