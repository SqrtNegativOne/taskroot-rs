import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render } from '@testing-library/svelte';
import { createDebouncedWriter, createStaleGuard } from './asyncState.svelte';
import HydrateHarness from './hydrateOnceHarness.svelte';

describe('hydrateOnce', () => {
    afterEach(() => {
        cleanup();
    });

    it('applies a load that resolves while the component is alive', async () => {
        const onApplied = vi.fn();
        render(HydrateHarness, {
            props: { load: () => Promise.resolve('loaded'), onApplied },
        });

        await vi.waitFor(() => {
            expect(onApplied).toHaveBeenCalledWith('loaded');
        });
    });

    it('drops a load response that resolves after destroy', async () => {
        const deferred = Promise.withResolvers<string>();
        const onApplied = vi.fn();
        const { unmount } = render(HydrateHarness, {
            props: { load: () => deferred.promise, onApplied },
        });

        unmount();
        deferred.resolve('late');
        await Promise.resolve();
        await Promise.resolve();
        expect(onApplied).not.toHaveBeenCalled();
    });
});

describe('createStaleGuard', () => {
    it('reports only the most recent request as current', () => {
        const guard = createStaleGuard();
        const first = guard.begin();
        const second = guard.begin();

        expect(first()).toBe(false);
        expect(second()).toBe(true);
    });

    it('expires the current request when invalidated', () => {
        const guard = createStaleGuard();
        const current = guard.begin();
        guard.invalidate();

        expect(current()).toBe(false);
    });
});

describe('createDebouncedWriter', () => {
    afterEach(() => {
        vi.useRealTimers();
    });

    it('coalesces rapid schedules into a single write of the latest value', () => {
        vi.useFakeTimers();
        const write = vi.fn();
        const writer = createDebouncedWriter(write, 10);

        writer.schedule(1);
        writer.schedule(2);
        writer.schedule(3);

        expect(write).not.toHaveBeenCalled();
        vi.advanceTimersByTime(10);
        expect(write).toHaveBeenCalledTimes(1);
        expect(write).toHaveBeenCalledWith(3);
    });

    it('flushes a pending write immediately without a second write later', () => {
        vi.useFakeTimers();
        const write = vi.fn();
        const writer = createDebouncedWriter(write, 10);

        writer.schedule(7);
        writer.flush();

        expect(write).toHaveBeenCalledWith(7);
        vi.advanceTimersByTime(10);
        expect(write).toHaveBeenCalledTimes(1);
    });

    it('does nothing when flushed with no pending write', () => {
        const write = vi.fn();
        const writer = createDebouncedWriter(write, 10);

        writer.flush();

        expect(write).not.toHaveBeenCalled();
    });
});
