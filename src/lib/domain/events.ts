import type { AppCalendar } from '../bindings/AppCalendar.generated';
import type { AppEvent } from '../bindings/AppEvent.generated';
import type { CalendarComponent } from '../bindings/CalendarComponent.generated';
import { addDays, ymd } from '../time';

const MS_PER_MINUTE = 60_000;
const DEFAULT_EVENT_MINUTES = 60;

export type CalendarLike = AppCalendar | {
    readonly id: string;
    readonly summary?: string;
    readonly color?: string;
    readonly backgroundColor?: string;
    readonly writable?: boolean;
    readonly component?: CalendarComponent;
};

export interface NewEventOptions {
    readonly startMinutes?: number;
    readonly endMinutes?: number;
    readonly calendars?: readonly CalendarLike[];
    /** Title of the draft; defaults to "New Event". */
    readonly title?: string;
    /** Task the new event is scheduled for, stored as the event's task link. */
    readonly taskId?: string;
}

function resolveEventCalendar(
    ev: { readonly remoteCollectionId?: string },
    calendars: readonly CalendarLike[] = [],
): CalendarLike | undefined {
    if (calendars.length === 0) {
        return undefined;
    }

    const fallback = calendars[0];
    const collectionId = ev.remoteCollectionId;
    if (!collectionId) {
        return fallback;
    }

    return calendars.find((c) => c.id === collectionId) ?? fallback;
}

export function isRecurring(event: {
    readonly rrule?: string;
    readonly recurringEventId?: string;
}): boolean {
    return event.rrule !== undefined || event.recurringEventId !== undefined;
}

export function isCalendarReadOnly(
    calendar: { readonly writable?: boolean } | undefined,
): boolean {
    return calendar?.writable === false;
}

function writableEventCalendar(
    calendars: readonly CalendarLike[],
): CalendarLike | undefined {
    return calendars.find(
        (calendar) => calendar.component === 'VEVENT' && !isCalendarReadOnly(calendar),
    );
}

/**
 * Builds an unsaved `AppEvent` draft for a date and optional time-of-day.
 * Omitting `startMinutes` creates an all-day event with floating dates; passing
 * it creates a timed event using the system timezone.
 */
export function buildNewEvent(
    date: Date,
    { startMinutes, endMinutes, calendars = [], title, taskId }: NewEventOptions = {},
): AppEvent {
    const calendar = writableEventCalendar(calendars);
    const isAllDay = startMinutes === undefined;
    const link = taskId ? { taskId } : {};

    if (isAllDay) {
        return {
            id: crypto.randomUUID(),
            title: title ?? 'New Event',
            startTime: ymd(date),
            endTime: ymd(addDays(date, 1)),
            remoteCollectionId: calendar?.id,
            isAllDay: true,
            ...link,
        };
    }

    const dayStart = new Date(date.getFullYear(), date.getMonth(), date.getDate());
    const end = endMinutes ?? startMinutes + DEFAULT_EVENT_MINUTES;

    return {
        id: crypto.randomUUID(),
        title: title ?? 'New Event',
        startTime: new Date(dayStart.getTime() + startMinutes * MS_PER_MINUTE).toISOString(),
        endTime: new Date(dayStart.getTime() + end * MS_PER_MINUTE).toISOString(),
        remoteCollectionId: calendar?.id,
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
    ev: { readonly remoteCollectionId?: string; readonly calendarId?: string },
    calendars: readonly CalendarLike[] = [],
): boolean {
    const collectionId = ev.remoteCollectionId ?? ev.calendarId;
    if (!collectionId) {
        return false;
    }
    return isCalendarReadOnly(resolveEventCalendar({ remoteCollectionId: collectionId }, calendars));
}


