import type { AppEventFilter } from './domain';

/** Local date window (both `YYYY-MM-DD`) accepted by `query_event_instances`. */
export interface EventInstanceRange {
    startDate: string;
    endDate: string;
}

/** Args for `query_event_instances`: render-ready instances in a local date window. */
export function eventInstances(range: EventInstanceRange, filters: AppEventFilter[], query: string) {
    return { filters, query, ...range };
}

/** Args for `query_events`: all raw events, unfiltered (inspector). */
export function allEvents() {
    return { filters: [] as AppEventFilter[], query: '' };
}

/** Args for `get_active_calendars`. */
export function activeCalendars() {
    return {};
}
