import { onDestroy } from 'svelte';
import { safeInvoke } from '../../lib/safeInvoke.svelte';
import { createDebouncedWriter, hydrateOnce } from '../../lib/asyncState.svelte';

/** Sidebar state lives in the backend `ui-state.json` under these keys. */
export const sidebarUiKeys = {
    tabTop: 'ui.sidebar.tab_top',
    notes: 'ui.sidebar.notes',
    showNotes: 'ui.sidebar.show_notes',
} as const;

const SETTLE_MS = 400;

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
 * Reactive sidebar state backed by `ui_state`. Hydrates once, then debounces
 * notes/visibility changes back to the backend and flushes them on destroy.
 * `tabTop` is persisted via [`saveTabTop`] when a drag ends, matching the old
 * write-on-pointer-up behaviour.
 */
export function createSidebarState(): SidebarState {
    const state = $state<SidebarState>({
        tabTop: 0,
        notes: '',
        showNotes: false,
        hydrated: false,
        storedTabTop: undefined,
    });
    let savedNotes = '';
    let savedShowNotes = false;

    const notesWriter = createDebouncedWriter<string>((value) => {
        savedNotes = value;
        void safeInvoke('set_ui_state', { key: sidebarUiKeys.notes, value });
    }, SETTLE_MS);
    const showNotesWriter = createDebouncedWriter<boolean>((value) => {
        savedShowNotes = value;
        void safeInvoke('set_ui_state', { key: sidebarUiKeys.showNotes, value });
    }, SETTLE_MS);

    hydrateOnce(
        async () => {
            const [tabTop, notes, showNotes] = await Promise.all([
                readSidebarValue(sidebarUiKeys.tabTop, isFiniteNumber),
                readSidebarValue(sidebarUiKeys.notes, isString),
                readSidebarValue(sidebarUiKeys.showNotes, isBoolean),
            ]);
            return { tabTop, notes, showNotes };
        },
        (loaded) => {
            state.storedTabTop = loaded.tabTop;
            if (loaded.tabTop !== undefined) {
                state.tabTop = loaded.tabTop;
            }
            if (loaded.notes !== undefined) {
                state.notes = loaded.notes;
                savedNotes = loaded.notes;
            }
            if (loaded.showNotes !== undefined) {
                state.showNotes = loaded.showNotes;
                savedShowNotes = loaded.showNotes;
            }
            state.hydrated = true;
        },
    );

    $effect(() => {
        if (!state.hydrated || state.notes === savedNotes) return;
        notesWriter.schedule(state.notes);
    });

    $effect(() => {
        if (!state.hydrated || state.showNotes === savedShowNotes) return;
        showNotesWriter.schedule(state.showNotes);
    });

    onDestroy(() => {
        notesWriter.flush();
        showNotesWriter.flush();
    });

    return state;
}

/** Persist the dragged tab position; called once when a drag ends. */
export function saveTabTop(tabTop: number): void {
    void safeInvoke('set_ui_state', { key: sidebarUiKeys.tabTop, value: tabTop });
}
