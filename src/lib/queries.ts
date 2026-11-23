import type { AppEventFilter, AppEventSort, AppTaskFilter, AppTaskSort, FilterGroup } from './domain';
import { emptyFilterGroup } from './domain';

/** Local date window (both `YYYY-MM-DD`) accepted by the render projections. */
export interface DateWindow {
    startDate: string;
    endDate: string;
}

/** Args for `query_event_instances`: render-ready instances in a local date window. */
export function eventInstances(
    range: DateWindow,
    filters: FilterGroup<AppEventFilter>,
    query: string,
    sort: AppEventSort[] = [],
) {
    return { filters, sort, query, ...range };
}

/** Args for `query_task_dates`: task start/due markers in a local date window. */
export function taskDates(range: DateWindow) {
    return { ...range };
}

/** Args for `query_events`: all raw events, unfiltered (inspector). */
export function allEvents() {
    return { filters: emptyFilterGroup<AppEventFilter>(), sort: [] as AppEventSort[], query: '' };
}

/** Args for `query_tasks`: all tasks, independent of the shared list filters. */
export function allTasks() {
    return { filters: emptyFilterGroup<AppTaskFilter>(), sort: [] as AppTaskSort[], query: '' };
}

/** Args for `get_active_calendars`. */
export function activeCalendars() {
    return {};
}
