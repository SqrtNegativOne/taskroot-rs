import { safeInvoke, type AppError } from './safeInvoke.svelte';
import { err, ok, ResultAsync, type Result } from 'neverthrow';
import { describeAppError, normalizeAppError, unknownAppError } from './errors';
import type { AppTask, AppEvent } from './domain';
import type { AppSettings } from './bindings/AppSettings.generated';

const MAX_INIT_ATTEMPTS = 50;
const INIT_RETRY_DELAY_MS = 100;

export function describeError(error: unknown): string {
    return describeAppError(error);
}

function isBackendNotReady(error: AppError): boolean {
    return error.code === 'not-ready';
}

function delay(ms: number): Promise<void> {
    return new Promise<void>((resolve) => setTimeout(resolve, ms));
}

export class AppStore {
    tasks = $state<AppTask[]>([]);
    events = $state<AppEvent[]>([]);
    settings = $state<AppSettings | null>(null);
    loaded = $state(false);
    error = $state<string | null>(null);

    private initPromise: Promise<Result<void, AppError>> | undefined;

    init(): Promise<Result<void, AppError>> {
        this.initPromise ??= this.bootstrap();
        return this.initPromise;
    }

    private async bootstrap(): Promise<Result<void, AppError>> {
        for (let attempt = 0; attempt < MAX_INIT_ATTEMPTS; attempt++) {
            const result = await this.refresh();
            if (result.isOk()) {
                this.loaded = true;
                return ok(undefined);
            }
            if (!isBackendNotReady(result.error)) return result;
            await delay(INIT_RETRY_DELAY_MS);
        }
        return err(unknownAppError('Database failed to initialize in time.'));
    }

    async refresh(): Promise<Result<void, AppError>> {
        const result = await ResultAsync.combine([
            safeInvoke<AppTask[]>('get_tasks'),
            safeInvoke<AppEvent[]>('get_events'),
            safeInvoke<any>('get_settings')
        ]);

        if (result.isErr()) {
            console.error('Failed to refresh store:', result.error);
            return err(normalizeAppError(result.error));
        }

        const [fetchedTasks, fetchedEvents, fetchedSettings] = result.value;
        this.tasks = fetchedTasks;
        this.events = fetchedEvents;
        this.settings = fetchedSettings;
        this.error = null;
        return ok(undefined);
    }

    addTask(task: AppTask): Promise<Result<void, AppError>> {
        return this.commit(safeInvoke('create_task', { task }));
    }

    updateTask(id: string, updater: (task: AppTask) => AppTask): Promise<Result<void, AppError>> {
        const current = this.tasks.find((task) => task.id === id);
        if (!current) return Promise.resolve(ok(undefined));
        return this.commit(safeInvoke('update_task', { task: updater(current) }));
    }

    deleteTask(id: string): Promise<Result<void, AppError>> {
        return this.commit(safeInvoke('delete_task', { id }));
    }

    addEvent(event: AppEvent): Promise<Result<void, AppError>> {
        return this.commit(safeInvoke('create_event', { event }));
    }

    updateEvent(id: string, updater: (event: AppEvent) => AppEvent): Promise<Result<void, AppError>> {
        const current = this.events.find((event) => event.id === id);
        if (!current) return Promise.resolve(ok(undefined));
        return this.commit(safeInvoke('update_event', { event: updater(current) }));
    }

    deleteEvent(id: string): Promise<Result<void, AppError>> {
        return this.commit(safeInvoke('delete_event', { id }));
    }

    private async commit(command: ResultAsync<unknown, AppError>): Promise<Result<void, AppError>> {
        const result = await command;
        if (result.isErr()) return err(result.error);
        const refreshResult = await this.refresh();
        if (refreshResult.isOk()) {
            import('@tauri-apps/api/event').then(({ emit }) => {
                void emit('store-updated');
            });
        }
        return refreshResult;
    }
}

export const store = new AppStore();

if (typeof window !== 'undefined') {
    import('@tauri-apps/api/event').then(({ listen }) => {
        void listen('store-updated', () => {
            void store.refresh();
        });
    });
}
