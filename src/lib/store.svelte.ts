import { safeInvoke, type AppError } from './safeInvoke.svelte';
import { err, ok, ResultAsync, type Result } from 'neverthrow';
import { describeAppError, normalizeAppError, unknownAppError } from './errors';
import { notify } from './notifications.svelte';
import type { AppTask, AppEvent } from './domain';
import type { AppSettings } from './bindings/AppSettings.generated';

export function describeError(error: unknown): string {
    return describeAppError(error);
}

export class AppStore {
    settings = $state<AppSettings | null>(null);
    loaded = $state(false);
    /** Fatal load/bootstrap failure; per-operation failures are surfaced as notifications. */
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
        // A webview can issue commands before Rust's `setup` hook has managed the
        // store, which answers `not-ready` until then. Retry that one briefly
        // instead of caching the failure and rendering "Loading..." forever.
        for (let attempt = 0; attempt < 40; attempt += 1) {
            const result = await this.refresh();
            if (result.isOk()) {
                this.loaded = true;
                return result;
            }
            if (result.error.code !== 'not-ready') return result;
            await new Promise((resolve) => setTimeout(resolve, 150));
        }
        return err(unknownAppError('The backend did not finish starting.'));
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

    /**
     * Run a mutation and report failure as a transient error notification.
     * `store.error` stays reserved for the fatal load/bootstrap path.
     */
    async mutate<T>(
        operation: string,
        action: () => Promise<Result<T, AppError>>,
    ): Promise<Result<T, AppError>> {
        const result = await action();
        if (result.isErr()) {
            notify(`${operation}: ${describeAppError(result.error)}`, 'error');
        }
        return result;
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

    createCalendar(
        displayName: string,
        component: 'VEVENT' | 'VTODO',
    ): Promise<Result<void, AppError>> {
        return this.commit(safeInvoke('create_calendar', { displayName, component }));
    }

    renameCalendar(href: string, displayName: string): Promise<Result<void, AppError>> {
        return this.commit(safeInvoke('rename_calendar', { href, displayName }));
    }

    setCalendarColor(href: string, color: string | null): Promise<Result<void, AppError>> {
        return this.commit(safeInvoke('set_calendar_color', { href, color }));
    }

    deleteCalendar(href: string): Promise<Result<void, AppError>> {
        return this.commit(safeInvoke('delete_calendar', { href }));
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
