import type { UnlistenFn } from '@tauri-apps/api/event';
import type { AppCalendar } from './domain';
import { createStaleGuard } from './asyncState.svelte';
import {
    commands,
    events,
    safeInvoke,
    type AppError,
    type AutoQueryResult,
} from './safeInvoke.svelte';

/**
 * Ref-counted window-level reactive query. The first active subscriber starts
 * one IPC fetch and one `StoreUpdated` listener; concurrent subscribers share
 * the same reactive state; when the last subscriber unmounts (or disables),
 * the listener tears down and state resets cleanly.
 */
class SharedQuery<T> implements AutoQueryResult<T, AppError> {
    private subscribers = 0;
    private _data = $state<T | undefined>(undefined);
    private _error = $state<AppError | undefined>(undefined);
    private _isLoading = $state(false);
    private active = false;
    private listenGen = 0;
    private unlisten: UnlistenFn | undefined;
    private readonly guard = createStaleGuard();

    constructor(private readonly command: () => Promise<T>) {}

    get data(): T | undefined {
        return this._data;
    }

    get error(): AppError | undefined {
        return this._error;
    }

    get isLoading(): boolean {
        return this._isLoading;
    }

    private async fetchNow(): Promise<void> {
        const isCurrent = this.guard.begin();
        this._isLoading = true;
        this._error = undefined;

        const result = await safeInvoke<T, AppError>(this.command());
        if (!this.active || !isCurrent()) return;

        if (result.isOk()) {
            this._data = result.value;
        } else {
            this._error = result.error;
        }
        this._isLoading = false;
    }

    private start(): void {
        this.active = true;
        const gen = ++this.listenGen;
        void this.fetchNow();
        void events.storeUpdated.listen(() => {
            if (this.active && gen === this.listenGen) void this.fetchNow();
        }).then((unlisten) => {
            if (!this.active || gen !== this.listenGen) {
                unlisten();
                return;
            }
            this.unlisten = unlisten;
        });
    }

    private stop(): void {
        this.active = false;
        this.listenGen += 1;
        this.guard.invalidate();
        this.unlisten?.();
        this.unlisten = undefined;
        this._data = undefined;
        this._error = undefined;
        this._isLoading = false;
    }

    subscribe(isEnabled?: () => boolean): AutoQueryResult<T, AppError> {
        $effect(() => {
            if (isEnabled && !isEnabled()) return;
            if (this.subscribers === 0) this.start();
            this.subscribers += 1;
            return () => {
                this.subscribers -= 1;
                if (this.subscribers === 0) this.stop();
            };
        });

        return this;
    }
}

const activeCalendarsQuery = new SharedQuery<AppCalendar[]>(() => commands.getActiveCalendars());
const tagsQuery = new SharedQuery<string[]>(() => commands.listTags());

/** Shared window-level reactive query for `commands.getActiveCalendars`. */
export function useActiveCalendars(
    isEnabled?: () => boolean,
): AutoQueryResult<AppCalendar[], AppError> {
    return activeCalendarsQuery.subscribe(isEnabled);
}

/** Shared window-level reactive query for `commands.listTags`. */
export function useTags(
    isEnabled?: () => boolean,
): AutoQueryResult<string[], AppError> {
    return tagsQuery.subscribe(isEnabled);
}
