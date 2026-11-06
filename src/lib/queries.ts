import type { AppEventFilter } from './domain';

/** Local date window (both `YYYY-MM-DD`) accepted by the render projections. */
export interface DateWindow {
    startDate: string;
    endDate: string;
}

/** Args for `query_event_instances`: render-ready instances in a local date window. */
export function eventInstances(range: DateWindow, filters: AppEventFilter[], query: string) {
    return { filters, query, ...range };
}

/** Args for `query_task_dates`: task start/due markers in a local date window. */
export function taskDates(range: DateWindow) {
    return { ...range };
}

/** Args for `query_events`: all raw events, unfiltered (inspector). */
export function allEvents() {
    return { filters: [] as AppEventFilter[], query: '' };
}

/** Args for `get_active_calendars`. */
export function activeCalendars() {
    return {};
}
