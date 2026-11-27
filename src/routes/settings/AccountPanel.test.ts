import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import AccountPanel from './AccountPanel.svelte';
import type { AccountRegistry } from '../../lib/bindings/AccountRegistry.generated';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();
let registry: AccountRegistry;

function profile(id: string, username: string) {
    return { id, account: { serverUrl: 'http://127.0.0.1:15232', username,
        principalHref: `/${username}/`, homeHref: `/${username}/`, calendars: [] } };
}

async function dummyRow() {
    return within(await screen.findByRole('region', { name: 'dummy on http://127.0.0.1:15232' }));
}

function invoked(command: string): boolean {
    return tauri.invoke.mock.calls.some(([name]) => name === command);
}

describe('AccountPanel', () => {
    beforeEach(() => {
        tauri.reset();
        registry = { activeId: 'real', profiles: [profile('real', 'normal'), profile('dummy', 'dummy')] };
        tauri.stubCommand('get_accounts', () => registry);
        tauri.stubCommandValue('get_active_calendars', []);
        tauri.stubCommand('switch_account', (args) => {
            if (typeof args?.accountId !== 'string') throw new Error('Expected accountId');
            registry = { ...registry, activeId: args.accountId };
        });
        tauri.stubCommand('remove_account', (args) => {
            if (typeof args?.accountId !== 'string') throw new Error('Expected accountId');
            registry = { activeId: registry.activeId === args.accountId ? null : registry.activeId,
                profiles: registry.profiles.filter((profile) => profile.id !== args.accountId) };
        });
        tauri.stubCommand('disconnect_account', () => { registry = { ...registry, activeId: null }; });
        vi.spyOn(window, 'confirm').mockReturnValue(true);
    });

    afterEach(() => {vi.restoreAllMocks(); });

    it('switches to a saved account in place without disconnecting or entering a password', async () => {
        render(AccountPanel);
        const row = await dummyRow();
        await fireEvent.click(row.getByRole('button', { name: 'Switch' }));
        await waitFor(() => expect(row.getByText('Active')).toBeTruthy());
        expect(tauri.invoke).toHaveBeenCalledWith('switch_account', { accountId: 'dummy' });
        expect(invoked('disconnect_account')).toBe(false);
        expect(screen.queryByLabelText('Password')).toBeNull();
        expect(screen.getByText('normal')).toBeTruthy();
    });

    it('keeps the previous active identity when a switch fails', async () => {
        tauri.stubCommand('switch_account', () => Promise.reject({ code: 'remote', message: 'Dummy server is unavailable' }));
        render(AccountPanel);
        const row = await dummyRow();
        await fireEvent.click(row.getByRole('button', { name: 'Switch' }));
        await screen.findByRole('alert');
        expect(row.queryByText('Active')).toBeNull();
        expect(within(screen.getByRole('region', { name: 'normal on http://127.0.0.1:15232' })).getByText('Active')).toBeTruthy();
        expect(row.getByRole('button', { name: 'Switch' }).hasAttribute('disabled')).toBe(false);
    });

    it('forgets only the confirmed profile and leaves the other account active', async () => {
        render(AccountPanel);
        await fireEvent.click((await dummyRow()).getByRole('button', { name: 'Forget' }));
        await waitFor(() => expect(screen.queryByText('dummy')).toBeNull());
        expect(tauri.invoke).toHaveBeenCalledWith('remove_account', { accountId: 'dummy' });
        expect(screen.getByText('Active')).toBeTruthy();
    });

    it('does not forget a profile when confirmation is declined', async () => {
        vi.mocked(window.confirm).mockReturnValue(false);
        render(AccountPanel);
        await fireEvent.click((await dummyRow()).getByRole('button', { name: 'Forget' }));
        expect(invoked('remove_account')).toBe(false);
        expect(screen.getByText('dummy')).toBeTruthy();
    });

    it('disconnects without removing saved profiles and can switch back immediately', async () => {
        render(AccountPanel);
        await screen.findByText('normal');
        await fireEvent.click(screen.getByRole('button', { name: 'Disconnect' }));
        await screen.findByText('No account connected. Choose a saved account or add one.');
        await fireEvent.click((await dummyRow()).getByRole('button', { name: 'Switch' }));
        await waitFor(() => expect((screen.getByText('Active'))).toBeTruthy());
        expect(screen.getByText('normal')).toBeTruthy();
        expect(invoked('remove_account')).toBe(false);
    });

    it('reconnects using an inline form without exposing any saved password', async () => {
        render(AccountPanel);
        await fireEvent.click((await dummyRow()).getByRole('button', { name: 'Reconnect' }));
        expect((screen.getByLabelText('Server URL') as HTMLInputElement).value).toBe('http://127.0.0.1:15232');
        expect((screen.getByLabelText('Username') as HTMLInputElement).value).toBe('dummy');
        expect((screen.getByLabelText('Password') as HTMLInputElement).value).toBe('');
        await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
        expect(screen.queryByLabelText('Password')).toBeNull();
    });
});
