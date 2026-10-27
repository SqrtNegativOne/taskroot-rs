import { vi, type Mock } from 'vitest';

/** Arguments bag passed as Tauri's second `invoke` parameter. */
export type InvokeArgs = Record<string, unknown>;

/** The `invoke(command, args)` shape mocked by this helper. */
export type InvokeFn = (command: string, args?: InvokeArgs) => Promise<unknown>;

/** Handler registered for a single command; receives the args bag. */
export type CommandHandler = (args: InvokeArgs | undefined) => unknown;

/** The runtime globals that the externalized `@tauri-apps/api` modules read. */
interface TauriInternals {
    transformCallback: (callback: unknown, once?: boolean) => number;
    unregisterCallback: (id: number) => void;
    invoke: (command: string, args?: unknown, options?: unknown) => Promise<unknown>;
    convertFileSrc: (path: string, protocol?: string) => string;
}

interface TauriEventInternals {
    unregisterListener: (event: string, eventId: number) => void;
}

type TauriGlobalWindow = Window & {
    __TAURI_INTERNALS__?: TauriInternals;
    __TAURI_EVENT_PLUGIN_INTERNALS__?: TauriEventInternals;
};

let nextCallbackId = 1;

/**
 * Installs the Tauri runtime globals that `@tauri-apps/api` reads directly.
 *
 * `@tauri-apps/api/event` is externalized under Vitest, so its `listen()` calls
 * (and the unlisten function it returns) bypass `vi.mock('@tauri-apps/api/core')`
 * and need these globals to exist. Calling this repeatedly replaces the globals
 * with an equivalent set, and unlisten stays a no-op that resolves cleanly.
 */
export function installTauriGlobals(): void {
    if (typeof window === 'undefined') return;

    const globals = window as TauriGlobalWindow;
    globals.__TAURI_INTERNALS__ = {
        transformCallback: () => nextCallbackId++,
        unregisterCallback: () => { /* noop */ },
        invoke: () => Promise.resolve(undefined),
        convertFileSrc: (path) => path,
    };
    globals.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
        unregisterListener: () => { /* noop */ },
    };
}

export interface TauriMock {
    /** The mocked `invoke`, for direct `expect(invoke)...` assertions. */
    readonly invoke: Mock<InvokeFn>;
    /** Registers a handler for `command`. Unregistered commands resolve to `undefined`. */
    stubCommand(command: string, handler: CommandHandler): void;
    /** Registers a fixed resolved value for `command`. */
    stubCommandValue(command: string, value: unknown): void;
    /** Clears recorded calls and handlers, then reinstalls the dispatcher. */
    reset(): void;
}

let singleton: TauriMock | undefined;

/**
 * Returns the process-wide Tauri IPC mock, shared between the test file and the
 * `vi.mock('@tauri-apps/api/core')` factory so both observe the same `invoke`.
 *
 * ```ts
 * import { vi } from 'vitest';
 * import { mockTauriInvoke } from '../test/tauriMock';
 *
 * // vi.mock must stay in the test file so Vitest can hoist it.
 * vi.mock('@tauri-apps/api/core', async () => {
 *     const { mockTauriInvoke } = await import('../test/tauriMock');
 *     return { invoke: mockTauriInvoke().invoke };
 * });
 *
 * const tauri = mockTauriInvoke();
 * tauri.stubCommandValue('get_ui_state', ['stored']);
 * ```
 */
export function mockTauriInvoke(): TauriMock {
    singleton ??= createTauriMock();
    return singleton;
}

function createTauriMock(): TauriMock {
    const handlers = new Map<string, CommandHandler>();
    const invoke = vi.fn<InvokeFn>();

    const dispatch: InvokeFn = (command, args) =>
        Promise.resolve(handlers.get(command)?.(args));

    function install(): void {
        invoke.mockImplementation(dispatch);
    }

    install();

    return {
        invoke,
        stubCommand(command, handler) {
            handlers.set(command, handler);
        },
        stubCommandValue(command, value) {
            handlers.set(command, () => value);
        },
        reset() {
            handlers.clear();
            invoke.mockReset();
            install();
        },
    };
}
