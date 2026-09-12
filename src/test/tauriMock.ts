import { vi, type Mock } from 'vitest';

/** Arguments bag passed as Tauri's second `invoke` parameter. */
export type InvokeArgs = Record<string, unknown>;

/** The `invoke(command, args)` shape mocked by this helper. */
export type InvokeFn = (command: string, args?: InvokeArgs) => Promise<unknown>;

/** Handler registered for a single command; receives the args bag. */
export type CommandHandler = (args: InvokeArgs | undefined) => unknown;

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
