import type { AppCalendar, AppEvent, CalendarComponent } from '../bindings/index.generated';
import { addDays, ymd } from '../time';

const DEFAULT_EVENT_MINUTES = 60;

export type CalendarLike = AppCalendar | {
    readonly id: string;
    readonly summary?: string;
    readonly color?: string | null;
    readonly backgroundColor?: string;
    readonly writable?: boolean;
    readonly component?: CalendarComponent;
};

export interface NewEventOptions {
    readonly startMinutes?: number;
    readonly endMinutes?: number;
    /** Title of the draft; defaults to "New Event". */
    readonly title?: string;
    /** Task the new event is scheduled for, stored as the event's task link. */
    readonly taskId?: string;
}

export function isRecurring(event: {
    readonly rrule?: string | null;
    readonly recurringEventId?: string | null;
}): boolean {
    return event.rrule != null || event.recurringEventId != null;
}

export function isCalendarReadOnly(
    calendar: { readonly writable?: boolean } | undefined,
): boolean {
    return calendar?.writable === false;
}

/** The calendars that store one component kind (events or tasks). */
export function calendarsForComponent<T extends CalendarLike>(
    calendars: readonly T[],
    component: CalendarComponent,
): T[] {
    return calendars.filter((calendar) => calendar.component === component);
}

/**
 * Resolves the default writable calendar for `component`, mirroring Rust's
 * `Store::default_calendar`: returns `preferred` when it names a writable
 * calendar of that component, otherwise the first writable calendar of that
 * component. Intended for UI previews and pre-checks; omit `remoteCollectionId`
 * when creating items so Rust resolves the target list atomically.
 */
export function resolveDefaultCalendar<T extends CalendarLike>(
    calendars: readonly T[],
    component: CalendarComponent,
    preferred?: string,
): T | undefined {
    const writable = calendarsForComponent(calendars, component).filter(
        (calendar) => !isCalendarReadOnly(calendar),
    );
    if (preferred !== undefined && preferred !== '') {
        const match = writable.find((calendar) => calendar.id === preferred);
        if (match !== undefined) return match;
    }
    return writable[0];
}

/**
 * Builds an unsaved `AppEvent` draft for a date and optional time-of-day.
 * Omitting `startMinutes` creates an all-day event with floating dates; passing
 * it creates a timed event using the system timezone. Leaves
 * `remoteCollectionId` unset so Rust resolves `default_event_list` atomically.
 */
export function buildNewEvent(
    date: Date,
    { startMinutes, endMinutes, title, taskId }: NewEventOptions = {},
): AppEvent {
    const isAllDay = startMinutes === undefined;
    const link = taskId ? { taskId } : {};

    if (isAllDay) {
        return {
            id: crypto.randomUUID(),
            title: title ?? 'New Event',
            startTime: ymd(date),
            endTime: ymd(addDays(date, 1)),
            isAllDay: true,
            ...link,
        };
    }

    const year = date.getFullYear();
    const month = date.getMonth();
    const day = date.getDate();
    const end = endMinutes ?? startMinutes + DEFAULT_EVENT_MINUTES;

    return {
        id: crypto.randomUUID(),
        title: title ?? 'New Event',
        startTime: new Date(year, month, day, 0, startMinutes).toISOString(),
        endTime: new Date(year, month, day, 0, end).toISOString(),
        isAllDay: false,
        timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
        ...link,
    };
}

/**
 * Whether an event lives in a calendar the signed-in user cannot write to.
 * Accepts both `AppEvent` (`remoteCollectionId`) and `EventInstance`
 * (`calendarId`). Events with no calendar are locally authored and editable.
 */
export function isEventReadOnly(
    ev: { readonly remoteCollectionId?: string | null; readonly calendarId?: string | null },
    calendars: readonly CalendarLike[] = [],
): boolean {
    const collectionId = ev.remoteCollectionId ?? ev.calendarId;
    if (!collectionId) {
        return false;
    }
    return isCalendarReadOnly(calendars.find((calendar) => calendar.id === collectionId));
}


