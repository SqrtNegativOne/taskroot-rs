import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import TaskListState from './TaskListState.svelte';
import { defaultTaskListFilters, reloadTaskListState, taskListQueryArgs, taskListState } from './taskList.svelte';
import { persistedKeys } from './persisted.svelte';
import { filterGroup } from './domain';
import type { AppTaskFilter, FilterGroup } from './domain';
import { mockTauriInvoke } from '../test/tauriMock';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});
const tauri = mockTauriInvoke();
const nested: FilterGroup<AppTaskFilter> = { operator: 'or', children: [
    { kind: 'condition', filter: { column: 'status', operator: 'is', value: ['next-up'] } },
    { kind: 'group', ...filterGroup<AppTaskFilter>([
        { column: 'priority', operator: 'is', value: ['4'] },
        { column: 'tags', operator: 'is', value: ['work'] },
    ]) },
] };

beforeEach(() => {
    tauri.reset();
    taskListState.filters = defaultTaskListFilters();
    taskListState.sort = [];
});
afterEach(() => { cleanup(); });

describe('shared task-list filters', () => {
    it('hydrates a nested tree and sends it unchanged to list queries', async () => {
        tauri.stubCommand('get_ui_state', args => args?.key === persistedKeys.taskListFilters ? nested : []);
        render(TaskListState);
        await waitFor(() => { expect(taskListQueryArgs().filters).toEqual(nested); });
        expect(tauri.invoke.mock.calls.filter(([command]) => command === 'set_ui_state')).toHaveLength(0);
    });

    it('flushes nested edits and reloads the same definition in another window', async () => {
        const stored = new Map<string, unknown>([[persistedKeys.taskListFilters, filterGroup([])]]);
        tauri.stubCommand('get_ui_state', args => stored.get(String(args?.key)));
        tauri.stubCommand('set_ui_state', args => { stored.set(String(args?.key), args?.value); });
        const mounted = render(TaskListState);
        await waitFor(() => { expect(taskListState.filters).toEqual(filterGroup([])); });
        taskListState.filters = nested;
        await tick();
        mounted.unmount();
        await waitFor(() => { expect(stored.get(persistedKeys.taskListFilters)).toEqual(nested); });
        taskListState.filters = defaultTaskListFilters();
        await reloadTaskListState();
        expect(taskListQueryArgs().filters).toEqual(nested);
    });

    it('rejects an obsolete flat array during hydration and reload', async () => {
        tauri.stubCommand('get_ui_state', args => args?.key === persistedKeys.taskListFilters ? [{ column: 'status', operator: 'is', value: ['done'] }] : []);
        render(TaskListState);
        await waitFor(() => { expect(tauri.invoke).toHaveBeenCalledWith('get_ui_state', { key: persistedKeys.taskListFilters }); });
        expect(taskListState.filters).toEqual(defaultTaskListFilters());
        taskListState.filters = nested;
        await reloadTaskListState();
        expect(taskListState.filters).toEqual(defaultTaskListFilters());
    });
});
