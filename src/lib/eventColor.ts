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
 * Inline CSS custom properties shared by every event render surface so the
 * date grid and day timeline cannot drift apart. Empty when the event has no
 * color, letting each surface fall back to its own default.
 */
export function eventColorVars(color?: string): string {
    if (!color) return '';
    return `--ev-color: ${color}; --ev-bg: ${color};`;
}
