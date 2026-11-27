import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import AccountBoundaryHarness from '../../test/AccountBoundaryHarness.svelte';
import { ACCOUNT_CHANGED } from '../../lib/ipcEvents';

const events = vi.hoisted(() => new Map<string, () => void>());
vi.mock('@tauri-apps/api/event', () => ({
    listen: async (name: string, callback: () => void) => {
        events.set(name, callback);
        return () => events.delete(name);
    },
}));

beforeEach(() => events.clear());
describe('account boundary', () => {
    it('discards old account drafts on an account-change event', async () => {
        render(AccountBoundaryHarness);
        await waitFor(() => expect(events.has(ACCOUNT_CHANGED)).toBe(true));
        await fireEvent.input(screen.getByLabelText('Draft'), { target: { value: 'old account draft' } });
        events.get(ACCOUNT_CHANGED)?.();
        await waitFor(() => expect((screen.getByLabelText('Draft') as HTMLInputElement).value).toBe(''));
    });

    it('keeps Settings controls mounted when preservation is requested', async () => {
        render(AccountBoundaryHarness, { preserve: true });
        await waitFor(() => expect(events.has(ACCOUNT_CHANGED)).toBe(true));
        const input = screen.getByLabelText('Draft') as HTMLInputElement;
        await fireEvent.input(input, { target: { value: 'settings state' } });
        events.get(ACCOUNT_CHANGED)?.();
        await waitFor(() => expect(screen.getByLabelText('Draft')).toBe(input));
        expect(input.value).toBe('settings state');
    });
});
