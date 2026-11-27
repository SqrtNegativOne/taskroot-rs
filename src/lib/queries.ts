import type { AppEventFilter, AppEventSort, AppTaskFilter, AppTaskSort, FilterGroup } from './domain';
import { emptyFilterGroup } from './domain';

/** Local date window (both `YYYY-MM-DD`) accepted by the render projections. */
export interface DateWindow {
    startDate: string;
    endDate: string;
}

/** Args for `commands.queryEventInstances`: render-ready instances in a local date window. */
export function eventInstances(
    range: DateWindow,
    filters: FilterGroup<AppEventFilter>,
    query: string,
    sort: AppEventSort[] = [],
): [FilterGroup<AppEventFilter>, AppEventSort[], string, string, string] {
    return [filters, sort, query, range.startDate, range.endDate];
}

/** Args for `commands.queryTaskDates`: task start/due markers in a local date window. */
export function taskDates(range: DateWindow): [string, string] {
    return [range.startDate, range.endDate];
}

/** Args for `commands.queryEvents`: all raw events, unfiltered (inspector). */
export function allEvents(): [FilterGroup<AppEventFilter>, AppEventSort[], string] {
    return [emptyFilterGroup<AppEventFilter>(), [], ''];
}

/** Args for `commands.queryTasks`: all tasks, independent of the shared list filters. */
export function allTasks(): [FilterGroup<AppTaskFilter>, AppTaskSort[], string] {
    return [emptyFilterGroup<AppTaskFilter>(), [], ''];
}
