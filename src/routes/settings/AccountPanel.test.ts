import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import AccountPanel from './AccountPanel.svelte';
import type { Account } from '../../lib/bindings/Account.generated';
import type { AppCalendar } from '../../lib/domain';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

const account: Account = {
    serverUrl: 'https://dav.example.com',
    username: 'me',
    principalHref: '/me/',
    homeHref: '/me/2f07/',
    calendars: [],
};

const journal: AppCalendar = {
    id: '/me/journal/',
    summary: 'Journal',
    writable: true,
    component: 'VEVENT',
};

const chores: AppCalendar = {
    id: '/me/chores/',
    summary: 'Chores',
    writable: true,
    component: 'VTODO',
};

function invoked(command: string): boolean {
    return tauri.invoke.mock.calls.some(([name]) => name === command);
}

describe('AccountPanel', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('get_account', account);
        tauri.stubCommandValue('get_active_calendars', [journal, chores]);
        vi.spyOn(window, 'confirm').mockReturnValue(true);
    });

    afterEach(() => {
        cleanup();
        vi.restoreAllMocks();
    });

    it('shows the identity once and leaves internal paths out', async () => {
        render(AccountPanel, { props: { onSignedOut: vi.fn() } });
        await screen.findByText('Sign out');

        expect(screen.getAllByText('me')).toHaveLength(1);
        expect(screen.getByText('https://dav.example.com')).not.toBeNull();
        expect(screen.queryByText(/caldav account/i)).toBeNull();
        expect(screen.queryByText(/2f07/)).toBeNull();
    });

    it('disconnects and reports the sign-out once confirmed', async () => {
        const onSignedOut = vi.fn();
        render(AccountPanel, { props: { onSignedOut } });
        await screen.findByText('Sign out');

        await fireEvent.click(screen.getByRole('button', { name: 'Sign out' }));

        await waitFor(() => expect(invoked('disconnect_account')).toBe(true));
        await waitFor(() => expect(onSignedOut).toHaveBeenCalledTimes(1));
    });

    it('stays connected when the sign-out is declined', async () => {
        vi.mocked(window.confirm).mockReturnValue(false);
        const onSignedOut = vi.fn();
        render(AccountPanel, { props: { onSignedOut } });
        await screen.findByText('Sign out');

        await fireEvent.click(screen.getByRole('button', { name: 'Sign out' }));

        await Promise.resolve();
        expect(invoked('disconnect_account')).toBe(false);
        expect(onSignedOut).not.toHaveBeenCalled();
    });
});
