import { invoke, type InvokeArgs } from '@tauri-apps/api/core';
import { ResultAsync } from 'neverthrow';

/**
 * A wrapper around Tauri's `invoke` that returns a `ResultAsync` from `neverthrow`.
 * This aligns frontend error handling with the Rust backend's `Result` type.
 *
 * @param cmd The Tauri command name
 * @param args The arguments for the command
 * @returns A ResultAsync containing the success data or the error
 */
export function safeInvoke<T, E = Error>(cmd: string, args?: InvokeArgs): ResultAsync<T, E> {
    return ResultAsync.fromPromise(
        invoke<T>(cmd, args),
        (e) => e as E
    );
}

/**
 * A reactive Svelte 5 Rune wrapper for executing Tauri commands.
 * It manages loading, data, and error states automatically.
 *
 * @param cmd The Tauri command name
 * @param args Default arguments for the command (optional)
 * @returns An object with reactive state and an execute function
 */
export function useTauriQuery<T, E = Error>(cmd: string, args?: InvokeArgs) {
    let data = $state<T | undefined>(undefined);
    let error = $state<E | undefined>(undefined);
    let isLoading = $state<boolean>(false);

    /**
     * Executes the Tauri command.
     * @param newArgs Optional overriding arguments
     */
    async function execute(newArgs?: InvokeArgs) {
        isLoading = true;
        error = undefined;
        
        const result = await safeInvoke<T, E>(cmd, newArgs ?? args);
        
        result.match(
            (v) => {
                data = v;
                isLoading = false;
            },
            (e) => {
                error = e;
                isLoading = false;
            }
        );
    }

    return {
        get data() { return data; },
        get error() { return error; },
        get isLoading() { return isLoading; },
        execute
    };
}
