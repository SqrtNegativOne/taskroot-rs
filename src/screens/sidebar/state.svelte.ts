import { onDestroy } from 'svelte';
import { safeInvoke } from '../../lib/safeInvoke.svelte';
import { createDebouncedWriter, hydrateOnce } from '../../lib/asyncState.svelte';

/**
 * Sidebar state now lives in the backend `ui_state` table. The `sidebar_*`
 * `localStorage` keys are legacy: each is read at most once, only when the
 * backend has no row yet, then removed.
 */
export const sidebarUiKeys = {
    tabTop: 'ui.sidebar.tab_top',
    notes: 'ui.sidebar.notes',
    showNotes: 'ui.sidebar.show_notes',
} as const;

const LEGACY_KEYS = {
    tabTop: 'sidebar_tab_top',
    notes: 'sidebar_notes',
    showNotes: 'sidebar_show_notes',
} as const;

const SETTLE_MS = 400;

/** A legacy `localStorage` value and the typed `ui_state` value it migrates to. */
export interface SidebarSeed<T> {
    uiKey: string;
    legacyKey: string;
    isValid: (value: unknown) => value is T;
    parse: (raw: string) => T | undefined;
}

export const sidebarSeeds = {
    tabTop: {
        uiKey: sidebarUiKeys.tabTop,
        legacyKey: LEGACY_KEYS.tabTop,
        isValid: (value: unknown): value is number =>
            typeof value === 'number' && Number.isFinite(value),
        parse: (raw: string) => {
            const value = Number.parseFloat(raw);
            return Number.isFinite(value) ? value : undefined;
        },
    },
    notes: {
        uiKey: sidebarUiKeys.notes,
        legacyKey: LEGACY_KEYS.notes,
        isValid: (value: unknown): value is string => typeof value === 'string',
        parse: (raw: string) => raw,
    },
    showNotes: {
        uiKey: sidebarUiKeys.showNotes,
        legacyKey: LEGACY_KEYS.showNotes,
        isValid: (value: unknown): value is boolean => typeof value === 'boolean',
        parse: (raw: string) => raw === 'true',
    },
};

/**
 * Reads one persisted sidebar value. When the backend has a row it wins and the
 * legacy key is never consulted; otherwise the value is seeded from the legacy
 * `localStorage` entry, written to the backend, and the legacy key removed.
 */
export async function readOrSeedSidebar<T>(seed: SidebarSeed<T>): Promise<T | undefined> {
    const result = await safeInvoke<unknown>('get_ui_state', { key: seed.uiKey });
    if (result.isErr()) return undefined;

    const stored = result.value;
    if (stored !== null && stored !== undefined) {
        return seed.isValid(stored) ? stored : undefined;
    }
    return seedFromLocalStorage(seed);
}

async function seedFromLocalStorage<T>(seed: SidebarSeed<T>): Promise<T | undefined> {
    const raw = localStorage.getItem(seed.legacyKey);
    if (raw === null) return undefined;
    localStorage.removeItem(seed.legacyKey);

    const value = seed.parse(raw);
    if (value === undefined) return undefined;
    await safeInvoke('set_ui_state', { key: seed.uiKey, value });
    return value;
}

export interface SidebarState {
    tabTop: number;
    notes: string;
    showNotes: boolean;
    /** True once the backend/legacy read has resolved. */
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
                readOrSeedSidebar(sidebarSeeds.tabTop),
                readOrSeedSidebar(sidebarSeeds.notes),
                readOrSeedSidebar(sidebarSeeds.showNotes),
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
