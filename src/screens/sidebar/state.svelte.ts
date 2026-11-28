import { commands, safeInvoke } from '../../lib/safeInvoke.svelte';
import { hydrateOnce } from '../../lib/asyncState.svelte';
import { persistState } from '../../lib/persisted.svelte';
import {
    computeSidebarBounds,
    createTauriSidebarWindowAdapter,
    SIDEBAR_CLOSE_TRANSITION_MS,
    SIDEBAR_TAB_HEIGHT,
    type SidebarMonitor,
    type SidebarWindowAdapter,
} from './windowAdapter';

export {
    computeSidebarBounds,
    createTauriSidebarWindowAdapter,
    SIDEBAR_CLOSE_TRANSITION_MS,
    SIDEBAR_PANEL_WIDTH,
    SIDEBAR_TAB_HEIGHT,
    SIDEBAR_TAB_WIDTH,
    type SidebarMonitor,
    type SidebarWindowAdapter,
    type SidebarWindowBounds,
} from './windowAdapter';

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
    const result = await safeInvoke(commands.getUiState(key));
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
    void safeInvoke(commands.setUiState(sidebarUiKeys.tabTop, tabTop));
}

export interface SidebarWindowControllerOptions {
    readonly adapter?: SidebarWindowAdapter;
    readonly closeTransitionMs?: number;
}

class SidebarWindowController {
    open = $state(false);
    windowIsSmall = $state(true);
    monitorReady = $state(false);

    private readonly adapter: SidebarWindowAdapter;
    private readonly closeTransitionMs: number;
    private monitor: SidebarMonitor | null = null;
    private closeTimer: ReturnType<typeof setTimeout> | undefined;
    private transitionGen = 0;
    private intentOpen = false;
    private disposed = false;

    constructor(options: SidebarWindowControllerOptions = {}) {
        this.adapter = options.adapter ?? createTauriSidebarWindowAdapter();
        this.closeTransitionMs = options.closeTransitionMs ?? SIDEBAR_CLOSE_TRANSITION_MS;
    }

    private clearCloseTimer(): void {
        if (this.closeTimer !== undefined) {
            clearTimeout(this.closeTimer);
            this.closeTimer = undefined;
        }
    }

    private isCurrent(gen: number): boolean {
        return !this.disposed && gen === this.transitionGen;
    }

    private async applyBounds(isOpen: boolean, tabTop: number, gen: number): Promise<boolean> {
        if (this.monitor === null || this.monitor.size.width === 0 || this.monitor.size.height === 0) {
            return this.isCurrent(gen);
        }
        const bounds = computeSidebarBounds(this.monitor, isOpen, tabTop);
        await this.adapter.setPosition(bounds.x, bounds.y);
        if (!this.isCurrent(gen)) return false;
        await this.adapter.setSize(bounds.width, bounds.height);
        return this.isCurrent(gen);
    }

    init = async (): Promise<void> => {
        await this.adapter.show();
        if (this.disposed) return;
        await this.adapter.unminimize();
        if (this.disposed) return;
        await this.adapter.setFocus();
        if (this.disposed) return;

        try {
            let found = await this.adapter.currentMonitor();
            if (this.disposed) return;
            found ??= await this.adapter.primaryMonitor();
            if (this.disposed || found === null) return;
            this.monitor = found;
            this.monitorReady = true;
        } catch (err) {
            console.error('Failed to get monitor:', err);
        }
    };

    defaultTabTop = (): number => {
        if (this.monitor === null) return 0;
        return (this.monitor.size.height / this.monitor.scaleFactor - SIDEBAR_TAB_HEIGHT) / 2;
    };

    clampTabTop = (tabTop: number): number => {
        if (this.monitor === null) return Math.max(0, tabTop);
        const maxTop = Math.max(
            0,
            this.monitor.size.height / this.monitor.scaleFactor - SIDEBAR_TAB_HEIGHT,
        );
        return Math.min(maxTop, Math.max(0, tabTop));
    };

    syncBounds = (tabTop: number): Promise<boolean> => {
        if (this.disposed) return Promise.resolve(false);
        return this.applyBounds(this.open, tabTop, this.transitionGen);
    };

    moveCollapsedTab = (tabTop: number): void => {
        if (this.disposed || !this.windowIsSmall || this.monitor === null) return;
        const bounds = computeSidebarBounds(this.monitor, false, tabTop);
        void this.adapter.setPosition(bounds.x, bounds.y);
    };

    toggle = async (tabTop: number): Promise<void> => {
        if (this.disposed) return;
        this.clearCloseTimer();
        const gen = ++this.transitionGen;

        if (!this.open && !this.intentOpen) {
            this.intentOpen = true;
            this.windowIsSmall = false;
            const ok = await this.applyBounds(true, tabTop, gen);
            if (!ok) return;
            this.open = true;
            return;
        }

        this.intentOpen = false;
        this.open = false;
        this.closeTimer = setTimeout(() => {
            this.closeTimer = undefined;
            if (!this.isCurrent(gen)) return;
            this.windowIsSmall = true;
            void this.applyBounds(false, tabTop, gen);
        }, this.closeTransitionMs);
    };

    dispose = (): void => {
        this.disposed = true;
        this.intentOpen = false;
        ++this.transitionGen;
        this.clearCloseTimer();
    };
}

export function createSidebarWindowController(options?: SidebarWindowControllerOptions) {
    return new SidebarWindowController(options);
}
