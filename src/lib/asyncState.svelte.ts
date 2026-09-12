/**
 * Small building blocks for keeping a reactive value in sync with the backend.
 *
 * These are deliberately not a framework: callers compose them inside a
 * component (or rune) and keep the state machine local, so the result stays
 * readable. See `persisted.svelte.ts` for the canonical composition.
 */

export interface StaleGuard {
    /** Starts a request and returns a predicate for whether it is still current. */
    begin: () => () => boolean;
    /** Expires the current request without starting a new one (use on teardown). */
    invalidate: () => void;
}

/**
 * Guards against out-of-order async responses.
 *
 * Every `begin()` invalidates earlier requests, so a late response can be
 * dropped instead of clobbering newer data: keep the returned predicate and
 * check it before applying a result.
 */
export function createStaleGuard(): StaleGuard {
    let latest = 0;
    return {
        begin(): () => boolean {
            const id = ++latest;
            return () => id === latest;
        },
        invalidate(): void {
            ++latest;
        },
    };
}

export interface DebouncedWriter<T> {
    /** Replaces the pending value and restarts the debounce timer. */
    schedule: (value: T) => void;
    /** Writes the pending value immediately, or does nothing when none is pending. */
    flush: () => void;
}

/**
 * Coalesces rapid writes into a single debounced write.
 *
 * Register `flush` with `onDestroy` so a pending write is not lost on teardown.
 */
export function createDebouncedWriter<T>(
    write: (value: T) => void,
    debounceMs: number,
): DebouncedWriter<T> {
    let pending: { value: T } | undefined;
    let timer: ReturnType<typeof setTimeout> | undefined;

    function flush(): void {
        if (timer !== undefined) {
            clearTimeout(timer);
            timer = undefined;
        }
        if (pending === undefined) return;
        const { value } = pending;
        pending = undefined;
        write(value);
    }

    function schedule(value: T): void {
        pending = { value };
        if (timer !== undefined) clearTimeout(timer);
        timer = setTimeout(() => {
            timer = undefined;
            flush();
        }, debounceMs);
    }

    return { schedule, flush };
}

/**
 * Hydrates once from an async loader on mount.
 *
 * `apply` runs only while the load is still current, so a teardown or a newer
 * load does not apply a late response. Read reactive values inside `load` to
 * re-hydrate when they change.
 */
export function hydrateOnce<T>(load: () => Promise<T>, apply: (value: T) => void): void {
    const guard = createStaleGuard();
    $effect(() => {
        const isCurrent = guard.begin();
        void load().then((value) => {
            if (!isCurrent()) return;
            apply(value);
        });
        return () => {
            guard.invalidate();
        };
    });
}
