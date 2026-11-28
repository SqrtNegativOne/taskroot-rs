import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render } from '@testing-library/svelte';
import EffectPage from './+page.svelte';

const state = vi.hoisted(() => ({
    resolveListen: undefined as ((unlisten: () => void) => void) | undefined,
}));

vi.mock('@tauri-apps/api/event', () => ({
    listen: () =>
        new Promise<() => void>((resolve) => {
            state.resolveListen = resolve;
        }),
}));

describe('effect route', () => {
    beforeEach(() => {
        state.resolveListen = undefined;
    });

    it('unsubscribes the screenEffect listener if unmounted before listen resolves', async () => {
        const unlisten = vi.fn();
        const view = render(EffectPage);
        expect(state.resolveListen).toBeDefined();

        view.unmount();
        state.resolveListen!(unlisten);
        await Promise.resolve();

        expect(unlisten).toHaveBeenCalledOnce();
    });
});
