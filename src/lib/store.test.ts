import { beforeEach, describe, expect, it } from 'vitest';
import { err, ok } from 'neverthrow';
import { AppStore } from './store.svelte';
import { notificationStore } from './notifications.svelte';
import type { AppError } from './errors';

const failure: AppError = { code: 'remote', message: 'server unreachable' };

describe('AppStore.mutate', () => {
    beforeEach(() => {
        notificationStore.notifications = [];
    });

    it('returns the failure and emits an error notification', async () => {
        const store = new AppStore();

        const result = await store.mutate('Failed to save', async () => err(failure));

        expect(result.isErr()).toBe(true);
        expect(notificationStore.notifications).toHaveLength(1);
        const notification = notificationStore.notifications[0];
        expect(notification?.type).toBe('error');
        expect(notification?.message).toBe('Failed to save: server unreachable');
        expect(store.error).toBeNull();
    });

    it('returns the success without notifying', async () => {
        const store = new AppStore();

        const result = await store.mutate('Failed to save', async () => ok('saved'));

        expect(result.isOk()).toBe(true);
        if (result.isOk()) expect(result.value).toBe('saved');
        expect(notificationStore.notifications).toHaveLength(0);
    });
});
