import { ResultAsync } from 'neverthrow';
import type { AppError } from './errors';
import { commands, events } from './bindings/index.generated';
import { createDebouncer, createStaleGuard } from './asyncState.svelte';

export { commands, events };
export type { AppError, AppErrorCode } from './errors';
export { describeAppError, normalizeAppError, unknownAppError } from './errors';

/**
 * Wraps a `tauri-specta` command promise in a `ResultAsync` from `neverthrow`,
 * aligning frontend error handling with the Rust backend's `Result<T, AppError>`.
 */
export function safeInvoke<T, E = AppError>(promise: Promise<T>): ResultAsync<T, E> {
    return ResultAsync.fromPromise(promise, (e) => e as E);
}

interface UseTauriQueryOptions<T, Args extends unknown[], E> {
    args?: Args;
    debounceMs?: number;
    enabled?: () => boolean;
    initialData?: T;
    onError?: (error: E) => void;
}

export interface AutoQueryResult<T, E = AppError> {
    readonly data: T | undefined;
    readonly error: E | undefined;
    readonly isLoading: boolean;
}

/**
 * A reactive Svelte 5 Rune wrapper for executing a typed `tauri-specta` command.
 * It manages loading, data, and error states automatically. Overlapping
 * executions are stale-guarded: only the most recently requested response is
 * applied. Pass `debounceMs` to coalesce rapid calls (e.g. search keystrokes).
 */
export function useTauriQuery<T, Args extends unknown[] = [], E = AppError>(
    command: (...args: Args) => Promise<T>,
    options: UseTauriQueryOptions<T, Args, E> = {},
) {
    let data = $state<T | undefined>(options.initialData);
    let error = $state<E | undefined>(undefined);
    let isLoading = $state(false);

    const guard = createStaleGuard();
    let lastArgs: Args | undefined;
    let resolveScheduled: (() => void) | undefined;

    const debouncer = options.debounceMs === undefined
        ? undefined
        : createDebouncer<Args>((args) => {
            const resolve = resolveScheduled;
            resolveScheduled = undefined;
            const settled = dispatch(args);
            if (resolve === undefined) {
                void settled;
                return;
            }
            void settled.finally(resolve);
        }, options.debounceMs);

    $effect(() => {
        if (options.enabled && !options.enabled()) {
            debouncer?.cancel();
            guard.invalidate();
            isLoading = false;
            return;
        }

        let isCleanedUp = false;
        let unlisten: import('@tauri-apps/api/event').UnlistenFn | undefined;

        void events.storeUpdated.listen(() => {
            if (isCleanedUp) return;
            const argsToUse = lastArgs ?? options.args;
            if (argsToUse !== undefined) {
                void dispatch(argsToUse);
            }
        }).then((un) => {
            unlisten = un;
            if (isCleanedUp) unlisten();
        });

        return () => {
            isCleanedUp = true;
            unlisten?.();
            debouncer?.cancel();
            guard.invalidate();
            resolveScheduled?.();
            resolveScheduled = undefined;
        };
    });

    async function dispatch(args: Args): Promise<void> {
        lastArgs = args;
        const isCurrent = guard.begin();
        isLoading = true;
        error = undefined;

        const result = await safeInvoke<T, E>(command(...args));
        if (!isCurrent()) return;

        result.match(
            (value) => {
                data = value;
                isLoading = false;
            },
            (errValue) => {
                error = errValue;
                isLoading = false;
            },
        );
        if (result.isErr()) options.onError?.(result.error);
    }

    async function execute(...args: Args): Promise<void> {
        const effective = args.length === 0 && options.args !== undefined ? options.args : args;
        lastArgs = effective;
        if (debouncer === undefined) return dispatch(effective);

        guard.invalidate();
        isLoading = true;
        resolveScheduled?.();
        resolveScheduled = undefined;
        const settled = new Promise<void>((resolve) => {
            resolveScheduled = resolve;
        });
        debouncer.schedule(effective);
        return settled;
    }

    return {
        get data() { return data; },
        get error() { return error; },
        get isLoading() { return isLoading; },
        execute,
    };
}

/**
 * A wrapper around `useTauriQuery` that automatically executes a typed command
 * whenever its reactive arguments change. `getArgs` is evaluated inside an
 * `$effect`, so Svelte tracks any `$state` variables read inside it.
 */
export function useAutoQuery<T>(
    command: () => Promise<T>,
    options?: Omit<UseTauriQueryOptions<T, [], AppError>, 'args'>,
): AutoQueryResult<T, AppError>;
export function useAutoQuery<T, Args extends unknown[]>(
    command: (...args: Args) => Promise<T>,
    getArgs: () => Args,
    options?: Omit<UseTauriQueryOptions<T, Args, AppError>, 'args'>,
): AutoQueryResult<T, AppError>;
export function useAutoQuery<T, Args extends unknown[]>(
    command: (...args: Args) => Promise<T>,
    getArgsOrOptions?: (() => Args) | Omit<UseTauriQueryOptions<T, Args, AppError>, 'args'>,
    maybeOptions?: Omit<UseTauriQueryOptions<T, Args, AppError>, 'args'>,
): AutoQueryResult<T, AppError> {
    const hasArgs = typeof getArgsOrOptions === 'function';
    const resolvedOptions = (hasArgs ? maybeOptions : getArgsOrOptions) ?? {};
    const query = useTauriQuery<T, Args>(command, resolvedOptions);
    $effect(() => {
        if (resolvedOptions.enabled && !resolvedOptions.enabled()) return;
        if (hasArgs) {
            void query.execute(...getArgsOrOptions());
        } else {
            void (query.execute as () => Promise<void>)();
        }
    });
    return {
        get data() { return query.data; },
        get error() { return query.error; },
        get isLoading() { return query.isLoading; },
    };
}
