import { createRawSnippet } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, waitFor } from '@testing-library/svelte';
import { goto } from '$app/navigation';
import { mockTauriInvoke } from '../test/tauriMock';
import Layout from './+layout.svelte';

const eventState = vi.hoisted(() => ({
    resolveNavListen: undefined as ((unlisten: () => void) => void) | undefined,
}));

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

vi.mock('@tauri-apps/api/window', () => ({
    getCurrentWindow: () => ({
        label: 'main',
        minimize: vi.fn(),
        toggleMaximize: vi.fn(),
        close: vi.fn(),
    }),
}));

vi.mock('@tauri-apps/api/event', () => ({
    listen: (name: string) => {
        if (name === 'launcher-navigate') {
            return new Promise<() => void>((resolve) => {
                eventState.resolveNavListen = resolve;
            });
        }
        return Promise.resolve(() => { /* noop */ });
    },
}));

vi.mock('$app/navigation', () => ({
    goto: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('$app/paths', () => ({
    resolve: (path: string) => path,
}));

vi.mock('$app/state', () => ({
    page: { url: { pathname: '/' } },
}));

const tauri = mockTauriInvoke();

const emptyChildren = createRawSnippet(() => ({
    render: () => '<div>child</div>',
}));

describe('root layout', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        tauri.reset();
        eventState.resolveNavListen = undefined;
        tauri.stubCommandValue('get_settings', { colour_scheme: 'slate' });
        tauri.stubCommandValue('get_accounts', {
            activeId: 'acc-1',
            profiles: [{ id: 'acc-1', account: { serverUrl: 'http://localhost', username: 'u', principalHref: '/', homeHref: '/', calendars: [] } }],
        });
        tauri.stubCommandValue('get_ui_state', '9999-12-31');
    });

    it('redirects to /connect when get_accounts has no active profile', async () => {
        tauri.stubCommandValue('get_accounts', { activeId: null, profiles: [] });
        render(Layout, { props: { children: emptyChildren } });

        await waitFor(() => {
            expect(goto).toHaveBeenCalledWith('/connect');
        });
    });

    it('unsubscribes launcherNavigate if unmounted before listen resolves', async () => {
        const unlisten = vi.fn();
        const view = render(Layout, { props: { children: emptyChildren } });

        await waitFor(() => {
            expect(eventState.resolveNavListen).toBeDefined();
        });

        view.unmount();
        eventState.resolveNavListen!(unlisten);

        await waitFor(() => {
            expect(unlisten).toHaveBeenCalledOnce();
        });
    });
});
