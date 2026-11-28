import { onDestroy, untrack } from 'svelte';
import type { JsonValue } from './domain';
import { commands, safeInvoke } from './safeInvoke.svelte';
import { createDebouncer, hydrateOnce } from './asyncState.svelte';

/**
 * Keys for per-component UI state persisted in the backend `ui-state.json` via
 * `get_ui_state`/`set_ui_state`. The `ui.` prefix is the persisted key
 * namespace; renaming a key orphans its stored value.
 */
export const persistedKeys = {
    appLastOpened: 'ui.app.last_opened',
    taskListFilters: 'ui.task_list.filters',
    taskListSort: 'ui.task_list.sort',
    dateGridFilters: 'ui.date_grid.filters',
    dateGridVisibility: 'ui.date_grid.visibility',
    dateGridView: 'ui.date_grid.view',
    dayTimelineFilters: 'ui.day_timeline.filters',
    dayTimelineNumDays: 'ui.day_timeline.num_days',
    sidebarTimelineFilters: 'ui.sidebar_timeline.filters',
    sidebarTimelineNumDays: 'ui.sidebar_timeline.num_days',
    planTaskPaneSize: 'ui.plan.task_pane_size',
    planCalendarSplitSize: 'ui.plan.calendar_split_size',
    doDistractionLogOpen: 'ui.do.distraction_log_open',
    doCurrentTasksOpen: 'ui.do.current_tasks_open',
    doTipsOpen: 'ui.do.tips_open',
    doNotesOpen: 'ui.do.notes_open',
    minitrackerPosition: 'ui.minitracker.position',
    wrapReflections: 'ui.wrap.reflections',
} as const;

const DEFAULT_DEBOUNCE_MS = 400;

function serialize(value: unknown): string {
    return JSON.stringify(value) ?? 'null';
}

function saveSetting(key: string, value: unknown): void {
    void safeInvoke(commands.setUiState(key, value as JsonValue));
}

interface PersistStateOptions {
    debounceMs?: number;
    isValid?: (value: unknown) => boolean;
    onHydrated?: () => void;
}

/**
 * Keeps a reactive value in sync with the backend `ui_state` store.
 *
 * The stored value is read once on mount and written back (debounced) whenever
 * `read()` changes. Writes are suppressed while hydration is in flight; if the
 * user edits in that window the local value wins and is saved instead of being
 * overwritten by the stored value. Unchanged values — including the component
 * fallback — are never written, so untouched defaults stay current with the
 * code and no redundant saves occur.
 *
 * @param key Backend `ui_state` key
 * @param read Reads the current reactive value
 * @param write Applies a hydrated value back into component state
 * @param options Debounce window, an optional validator and a hydration hook
 */
export function persistState<T>(
    key: string,
    read: () => T,
    write: (value: T) => void,
    options: PersistStateOptions = {},
): void {
    const debounceMs = options.debounceMs ?? DEFAULT_DEBOUNCE_MS;
    let hydrated = $state(false);
    let saved = '';
    const writer = createDebouncer<T>((value) => {
        saved = serialize(value);
        saveSetting(key, value);
    }, debounceMs);
    const fallback = serialize(untrack(() => $state.snapshot(read())) as T);

    hydrateOnce(
        async () => {
            const result = await safeInvoke(commands.getUiState(key));
            return result.isOk() ? result.value : null;
        },
        (stored) => {
            const present = stored !== null && stored !== undefined;
            const editedLocally = serialize($state.snapshot(read()) as T) !== fallback;
            if (editedLocally) {
                saved = fallback;
            } else if (present && (options.isValid?.(stored) ?? true)) {
                write(stored as T);
                saved = serialize(stored);
            } else {
                saved = fallback;
            }
            hydrated = true;
            options.onHydrated?.();
        },
    );

    $effect(() => {
        const snapshot = $state.snapshot(read()) as T;
        if (!hydrated) return;
        const serialized = serialize(snapshot);
        if (serialized === saved) {
            writer.cancel();
            return;
        }
        writer.schedule(snapshot);
    });

    onDestroy(writer.flush);
}
