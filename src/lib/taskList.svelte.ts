import type { AppTaskFilter, AppTaskSort, FilterGroup } from './domain';
import { filterGroup, isTaskFilterGroup } from './domain';
import { persistedKeys } from './persisted.svelte';
import { safeInvoke } from './safeInvoke.svelte';

export function defaultTaskListFilters(): FilterGroup<AppTaskFilter> {
    return filterGroup([{ column: 'status', operator: 'is not', value: ['done'] }]);
}

/**
 * The single task list definition shared by the plan pane, the Do screen and
 * the mini tracker. Filters and sort are persisted through `TaskListState`.
 */
export const taskListState = $state({
    filters: defaultTaskListFilters(),
    sort: [] as AppTaskSort[],
});

/** Arguments for `query_tasks` using the shared list definition. */
export function taskListQueryArgs() {
    return { filters: taskListState.filters, sort: taskListState.sort, query: '' };
}

function applyStoredFilters(value: unknown): void {
    taskListState.filters = isTaskFilterGroup(value) ? value : defaultTaskListFilters();
}

function applyStoredSort(value: unknown): void {
    if (Array.isArray(value)) taskListState.sort = value as AppTaskSort[];
}

/** Reload the persisted definition, for windows that outlive the editor. */
export async function reloadTaskListState(): Promise<void> {
    const [filters, sort] = await Promise.all([
        safeInvoke<unknown>('get_ui_state', { key: persistedKeys.taskListFilters }),
        safeInvoke<unknown>('get_ui_state', { key: persistedKeys.taskListSort }),
    ]);

    if (filters.isOk()) applyStoredFilters(filters.value);
    if (sort.isOk()) applyStoredSort(sort.value);
}
