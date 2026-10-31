import { safeInvoke } from '../../lib/safeInvoke.svelte';
import { hydrateOnce } from '../../lib/asyncState.svelte';
import { persistState } from '../../lib/persisted.svelte';

/** Sidebar state lives in the backend `ui-state.json` under these keys. */
export const sidebarUiKeys = {
    tabTop: 'ui.sidebar.tab_top',
    notes: 'ui.sidebar.notes',
    showNotes: 'ui.sidebar.show_notes',
} as const;

function isFiniteNumber(value: unknown): value is number {
    return typeof value === 'number' && Number.isFinite(value);
}

function isString(value: unknown): value is string {
    return typeof value === 'string';
}

function isBoolean(value: unknown): value is boolean {
    return typeof value === 'boolean';
}

async function readSidebarValue<T>(
    key: string,
    isValid: (value: unknown) => value is T,
): Promise<T | undefined> {
    const result = await safeInvoke<unknown>('get_ui_state', { key });
    if (result.isErr()) return undefined;
    return isValid(result.value) ? result.value : undefined;
}

export interface SidebarState {
    tabTop: number;
    notes: string;
    showNotes: boolean;
    /** True once the backend read has resolved. */
    hydrated: boolean;
    /** The stored tab position, or `undefined` when the backend had none. */
    storedTabTop: number | undefined;
}

/**
 * Reactive sidebar state backed by `ui_state`. Hydrates once, then persists
 * notes and visibility through `persistState` and flushes pending writes on
 * destroy. `tabTop` is persisted via [`saveTabTop`] when a drag ends, matching
 * the old write-on-pointer-up behaviour.
 */
export function createSidebarState(): SidebarState {
    const state = $state<SidebarState>({
        tabTop: 0,
        notes: '',
        showNotes: false,
        hydrated: false,
        storedTabTop: undefined,
    });

    let pendingHydrations = 3;
    function markHydrated(): void {
        pendingHydrations -= 1;
        if (pendingHydrations === 0) state.hydrated = true;
    }

    persistState(
        sidebarUiKeys.notes,
        () => state.notes,
        (value) => {
            state.notes = value;
        },
        { isValid: isString, onHydrated: markHydrated },
    );

    persistState(
        sidebarUiKeys.showNotes,
        () => state.showNotes,
        (value) => {
            state.showNotes = value;
        },
        { isValid: isBoolean, onHydrated: markHydrated },
    );

    hydrateOnce(
        () => readSidebarValue(sidebarUiKeys.tabTop, isFiniteNumber),
        (tabTop) => {
            state.storedTabTop = tabTop;
            if (tabTop !== undefined) {
                state.tabTop = tabTop;
            }
            markHydrated();
        },
    );

    return state;
}

/** Persist the dragged tab position; called once when a drag ends. */
export function saveTabTop(tabTop: number): void {
    void safeInvoke('set_ui_state', { key: sidebarUiKeys.tabTop, value: tabTop });
}
