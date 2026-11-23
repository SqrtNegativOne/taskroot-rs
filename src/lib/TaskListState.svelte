<script lang="ts">
    import type { AppTaskFilter, AppTaskSort, FilterGroup } from './domain';
    import { isTaskFilterGroup } from './domain';
    import { persistState, persistedKeys } from './persisted.svelte';
    import { taskListState } from './taskList.svelte';

    persistState<FilterGroup<AppTaskFilter>>(
        persistedKeys.taskListFilters,
        () => taskListState.filters,
        (stored) => {
            taskListState.filters = stored;
        },
        { isValid: isTaskFilterGroup },
    );

    persistState<AppTaskSort[]>(
        persistedKeys.taskListSort,
        () => taskListState.sort,
        (stored) => {
            taskListState.sort = stored;
        },
        { isValid: Array.isArray },
    );
</script>
