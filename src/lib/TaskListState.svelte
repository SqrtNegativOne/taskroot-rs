<script lang="ts">
    import type { AppTaskFilter, AppTaskSort } from './domain';
    import { persistState, persistedKeys } from './persisted.svelte';
    import { taskListState } from './taskList.svelte';

    persistState<AppTaskFilter[]>(
        persistedKeys.taskListFilters,
        () => taskListState.filters,
        (stored) => {
            taskListState.filters = stored;
        },
        { isValid: Array.isArray },
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
