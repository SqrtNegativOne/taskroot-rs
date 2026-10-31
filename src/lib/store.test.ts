import { beforeEach, describe, expect, it, vi } from 'vitest';
import { err, ok } from 'neverthrow';
import { toast } from 'svelte-sonner';
import { AppStore } from './store.svelte';
import type { AppError } from './errors';

vi.mock('svelte-sonner', () => ({
    toast: {
        error: vi.fn(),
        success: vi.fn(),
        info: vi.fn(),
        custom: vi.fn(),
    },
    Toaster: vi.fn(),
}));

const failure: AppError = { code: 'remote', message: 'server unreachable' };

describe('AppStore.mutate', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    it('returns the failure and emits an error notification', async () => {
        const store = new AppStore();

        const result = await store.mutate('Failed to save', async () => err(failure));

        expect(result.isErr()).toBe(true);
        expect(toast.custom).toHaveBeenCalledTimes(1);
        expect(toast.custom).toHaveBeenCalledWith(
            expect.anything(),
            expect.objectContaining({
                componentProps: { message: 'Failed to save: server unreachable' },
                duration: Infinity,
            }),
        );
        expect(store.error).toBeNull();
    });

    it('returns the success without notifying', async () => {
        const store = new AppStore();

        const result = await store.mutate('Failed to save', async () => ok('saved'));

        expect(result.isOk()).toBe(true);
        if (result.isOk()) expect(result.value).toBe('saved');
        expect(toast.custom).not.toHaveBeenCalled();
    });
});
