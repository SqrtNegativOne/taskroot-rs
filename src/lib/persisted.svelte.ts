import { onDestroy, untrack } from 'svelte';
import { safeInvoke } from './safeInvoke.svelte';

/**
 * Keys for per-component UI state persisted in the backend `settings` table.
 * Keys are namespaced under `ui.` so they never collide with `AppSettings`.
 */
export const persistedKeys = {
    taskListFilters: 'ui.task_list.filters',
    taskListSort: 'ui.task_list.sort',
    dateGridFilters: 'ui.date_grid.filters',
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
} as const;

const DEFAULT_DEBOUNCE_MS = 400;

function serialize(value: unknown): string {
    return JSON.stringify(value) ?? 'null';
}

function saveSetting(key: string, value: unknown): void {
    void safeInvoke('update_setting', { key, value });
}

interface PersistStateOptions {
    debounceMs?: number;
    isValid?: (value: unknown) => boolean;
}

interface PendingWrite<T> {
    value: T;
    serialized: string;
}

/**
 * Keeps a reactive value in sync with the backend settings store.
 *
 * The stored value is read once on mount and written back (debounced) whenever
 * `read()` changes. While hydration is in flight writes are suppressed so a
 * slow read cannot clobber a faster user edit. Unchanged values — including the
 * component fallback — are never written, so untouched defaults stay current
 * with the code and no redundant saves occur.
 *
 * @param key Backend settings key
 * @param read Reads the current reactive value
 * @param write Applies a hydrated value back into component state
 * @param options Debounce window and an optional validator for stored data
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
    let pending: PendingWrite<T> | undefined;
    let timer: ReturnType<typeof setTimeout> | undefined;

    function flush(): void {
        if (timer !== undefined) {
            clearTimeout(timer);
            timer = undefined;
        }
        if (pending === undefined) return;
        const entry = pending;
        pending = undefined;
        saved = entry.serialized;
        saveSetting(key, entry.value);
    }

    $effect(() => {
        let cancelled = false;
        const fallback = serialize(untrack(() => $state.snapshot(read())) as T);
        void (async () => {
            const result = await safeInvoke<unknown>('get_setting', { key });
            if (cancelled) return;
            const stored = result.isOk() ? result.value : null;
            const present = stored !== null && stored !== undefined;
            if (present && (options.isValid?.(stored) ?? true)) {
                write(stored as T);
                saved = serialize(stored);
            } else {
                saved = fallback;
            }
            hydrated = true;
        })();
        return () => {
            cancelled = true;
        };
    });

    $effect(() => {
        const snapshot = $state.snapshot(read()) as T;
        if (!hydrated) return;
        const serialized = serialize(snapshot);
        if (serialized === saved) return;
        pending = { value: snapshot, serialized };
        if (timer !== undefined) clearTimeout(timer);
        timer = setTimeout(() => {
            timer = undefined;
            flush();
        }, debounceMs);
    });

    onDestroy(flush);
}
