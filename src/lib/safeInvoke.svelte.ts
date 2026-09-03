import { invoke, type InvokeArgs } from '@tauri-apps/api/core';
import { ResultAsync } from 'neverthrow';
import type { AppError } from './errors';

export type { AppError, AppErrorCode } from './errors';
export { describeAppError, normalizeAppError, unknownAppError } from './errors';

/**
 * A wrapper around Tauri's `invoke` that returns a `ResultAsync` from `neverthrow`.
 * This aligns frontend error handling with the Rust backend's `Result` type.
 *
 * @param cmd The Tauri command name
 * @param args The arguments for the command
 * @returns A ResultAsync containing the success data or the error
 */
export function safeInvoke<T, E = AppError>(cmd: string, args?: InvokeArgs): ResultAsync<T, E> {
    return ResultAsync.fromPromise(
        invoke<T>(cmd, args),
        (e) => e as E
    );
}

/**
 * Declares reactive dependencies for a `$effect` that drives a `useTauriQuery`
 * execution. Svelte tracks any value read here, so the effect re-runs whenever
 * one of them changes — without resorting to bare `void x` statements.
 */
export function queryDependency(...dependencies: unknown[]): number {
    return dependencies.reduce<number>((count, value) => (value !== undefined ? count + 1 : count), 0);
}

interface UseTauriQueryOptions<T, E> {
    args?: InvokeArgs;
    debounceMs?: number;
    initialData?: T;
    onError?: (error: E) => void;
}

/**
 * A reactive Svelte 5 Rune wrapper for executing Tauri commands.
 * It manages loading, data, and error states automatically. Overlapping
 * executions are stale-guarded: only the most recently requested response is
 * applied. Pass `debounceMs` to coalesce rapid calls (e.g. search keystrokes).
 *
 * @param cmd The Tauri command name
 * @param options Default arguments, debounce window, initial data and an error hook
 * @returns An object with reactive state and an execute function
 */
export function useTauriQuery<T, E = AppError>(cmd: string, options: UseTauriQueryOptions<T, E> = {}) {
    let data = $state<T | undefined>(options.initialData);
    let error = $state<E | undefined>(undefined);
    let isLoading = $state(false);

    let latestRequestId = 0;
    let inflight: Promise<void> | undefined;
    let debounceHandle: ReturnType<typeof setTimeout> | undefined;

    let lastArgs: InvokeArgs | undefined;
    let unlisten: import('@tauri-apps/api/event').UnlistenFn | undefined;

    $effect(() => {
        let isCleanedUp = false;
        
        void import('@tauri-apps/api/event').then(({ listen }) => {
            void listen('store-updated', () => {
                if (isCleanedUp) return;
                const argsToUse = lastArgs ?? options.args;
                if (argsToUse !== undefined || latestRequestId > 0) {
                    void dispatch(argsToUse);
                }
            }).then((un) => {
                unlisten = un;
                if (isCleanedUp) unlisten();
            });
        });

        return () => {
            isCleanedUp = true;
            if (unlisten) unlisten();
        };
    });

    async function dispatch(args: InvokeArgs | undefined): Promise<void> {
        lastArgs = args;
        const requestId = ++latestRequestId;
        isLoading = true;
        error = undefined;

        inflight = (async () => {
            const result = await safeInvoke<T, E>(cmd, args ?? options.args);
            if (requestId !== latestRequestId) return;

            result.match(
                (value) => {
                    data = value;
                    isLoading = false;
                },
                (errValue) => {
                    error = errValue;
                    isLoading = false;
                }
            );
            if (result.isErr()) options.onError?.(result.error);
        })();

        await inflight;
    }

    function scheduleDispatch(args: InvokeArgs | undefined): Promise<void> {
        if (debounceHandle !== undefined) clearTimeout(debounceHandle);
        isLoading = true;

        return new Promise<void>((resolve) => {
            debounceHandle = setTimeout(() => {
                debounceHandle = undefined;
                void dispatch(args).finally(resolve);
            }, options.debounceMs ?? 0);
        });
    }

    /**
     * Executes the Tauri command.
     * @param newArgs Optional overriding arguments
     */
    async function execute(newArgs?: InvokeArgs): Promise<void> {
        lastArgs = newArgs;
        if (options.debounceMs === undefined) return dispatch(newArgs);
        return scheduleDispatch(newArgs);
    }

    return {
        get data() { return data; },
        get error() { return error; },
        get isLoading() { return isLoading; },
        execute
    };
}

/**
 * A wrapper around `useTauriQuery` that automatically executes the query
 * whenever its arguments change. `getArgs` is evaluated inside an `$effect`,
 * meaning Svelte automatically tracks any `$state` variables used inside it.
 *
 * @param cmd The Tauri command name
 * @param getArgs A function returning the arguments for the command
 * @param options Query options
 * @returns An object with reactive state properties
 */
export function useAutoQuery<T, Args extends InvokeArgs = InvokeArgs>(
    cmd: string,
    getArgs: () => Args,
    options: Omit<UseTauriQueryOptions<T, AppError>, 'args'> = {}
) {
    const query = useTauriQuery<T>(cmd, options);

    $effect(() => {
        void query.execute(getArgs());
    });

    return {
        get data() { return query.data; },
        get error() { return query.error; },
        get isLoading() { return query.isLoading; }
    };
}
