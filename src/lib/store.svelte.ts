import { safeInvoke, type AppError } from './safeInvoke.svelte';
import { err, ok, ResultAsync, type Result } from 'neverthrow';
import { describeAppError, normalizeAppError } from './errors';
import type { AppTask, AppEvent } from './domain';
import type { AppSettings } from './bindings/AppSettings.generated';

export function describeError(error: unknown): string {
    return describeAppError(error);
}

export class AppStore {
    settings = $state<AppSettings | null>(null);
    loaded = $state(false);
    error = $state<string | null>(null);

    private initPromise: Promise<Result<void, AppError>> | undefined;

    // Idempotent settings bootstrap, cached as a promise so overlapping callers
    // share one load. It deliberately does not use `hydrateOnce`: that wraps a
    // component `$effect`, and this store is a module-level singleton outside any
    // component. (`init()` no longer retries; the old `'not-ready'` loop is gone.)
    init(): Promise<Result<void, AppError>> {
        this.initPromise ??= this.bootstrap();
        return this.initPromise;
    }

    private async bootstrap(): Promise<Result<void, AppError>> {
        const result = await this.refresh();
        if (result.isOk()) {
            this.loaded = true;
        }
        return result;
    }

    async refresh(): Promise<Result<void, AppError>> {
        const result = await safeInvoke<AppSettings>('get_settings');

        if (result.isErr()) {
            console.error('Failed to refresh settings:', result.error);
            return err(normalizeAppError(result.error));
        }

        this.settings = result.value;
        this.error = null;
        return ok(undefined);
    }

    addTask(task: AppTask): Promise<Result<void, AppError>> {
        return this.commit(safeInvoke('create_task', { task }));
    }

    updateTask(task: AppTask): Promise<Result<void, AppError>> {
        return this.commit(safeInvoke('update_task', { task }));
    }

    deleteTask(id: string): Promise<Result<void, AppError>> {
        return this.commit(safeInvoke('delete_task', { id }));
    }

    addEvent(event: AppEvent): Promise<Result<void, AppError>> {
        return this.commit(safeInvoke('create_event', { event }));
    }

    updateEvent(event: AppEvent): Promise<Result<void, AppError>> {
        return this.commit(safeInvoke('update_event', { event }));
    }

    /** Reschedule a dragged instance back onto its master event. */
    rescheduleEvent(
        id: string,
        startTime: string,
        endTime: string,
        isAllDay = false,
    ): Promise<Result<void, AppError>> {
        return this.commit(
            safeInvoke('reschedule_event', { id, startTime, endTime, isAllDay }),
        );
    }

    deleteEvent(id: string): Promise<Result<void, AppError>> {
        return this.commit(safeInvoke('delete_event', { id }));
    }

    private async commit(command: ResultAsync<unknown, AppError>): Promise<Result<void, AppError>> {
        const result = await command;
        if (result.isErr()) return err(result.error);
        void import('@tauri-apps/api/event').then(({ emit }) => {
            void emit('store-updated');
        });
        return ok(undefined);
    }
}

export const store = new AppStore();

if (typeof window !== 'undefined') {
    void import('@tauri-apps/api/event').then(({ listen }) => {
        void listen('store-updated', () => {
            void store.refresh();
        });
    });
}
