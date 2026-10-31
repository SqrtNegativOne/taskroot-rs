import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke, type InvokeArgs } from '../test/tauriMock';
import CalendarManager from './CalendarManager.svelte';
import type { AppCalendar } from '../lib/domain';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

const journal: AppCalendar = {
    id: '/me/journal/',
    summary: 'Journal',
    color: '#ff000080',
    writable: true,
    component: 'VEVENT',
};

const chores: AppCalendar = {
    id: '/me/chores/',
    summary: 'Chores',
    writable: true,
    component: 'VTODO',
};

const shared: AppCalendar = {
    id: '/shared/',
    summary: 'Shared',
    writable: false,
    component: 'VTODO',
};

function invokedArgs(command: string): InvokeArgs | undefined {
    return tauri.invoke.mock.calls.find(([name]) => name === command)?.[1];
}

describe('CalendarManager', () => {
    beforeEach(() => {
        tauri.reset();
        vi.spyOn(window, 'confirm').mockReturnValue(true);
    });

    afterEach(() => {
        cleanup();
        vi.restoreAllMocks();
    });

    it('labels each list by what it actually holds', async () => {
        render(CalendarManager, { props: { calendars: [journal, chores] } });

        const items = screen.getAllByRole('listitem');
        expect(items).toHaveLength(2);
        expect(items[0]?.textContent).toContain('Events');
        expect(items[1]?.textContent).toContain('Tasks');
        expect(screen.getByLabelText('Name of Journal')).not.toBeNull();
        expect(screen.getByLabelText('Name of Chores')).not.toBeNull();
    });

    it('marks a read-only list and disables its editors', async () => {
        render(CalendarManager, { props: { calendars: [shared] } });

        expect((screen.getByLabelText('Name of Shared') as HTMLInputElement).disabled).toBe(true);
        expect((screen.getByLabelText('Delete Shared') as HTMLButtonElement).disabled).toBe(true);
        expect(screen.getByText(/read-only/)).not.toBeNull();
    });

    it('creates a list with the chosen name and kind', async () => {
        render(CalendarManager, { props: { calendars: [] } });

        await fireEvent.input(screen.getByPlaceholderText('New list name'), {
            target: { value: 'Holidays' },
        });
        await fireEvent.change(screen.getByRole('combobox'), { target: { value: 'VEVENT' } });
        await fireEvent.click(screen.getByRole('button', { name: 'Add' }));

        await waitFor(() =>
            expect(invokedArgs('create_calendar')).toEqual({
                displayName: 'Holidays',
                component: 'VEVENT',
            }),
        );
    });

    it('renames a list when the name field changes', async () => {
        render(CalendarManager, { props: { calendars: [journal] } });

        await fireEvent.change(screen.getByLabelText('Name of Journal'), {
            target: { value: 'Diary' },
        });

        await waitFor(() =>
            expect(invokedArgs('rename_calendar')).toEqual({
                href: '/me/journal/',
                displayName: 'Diary',
            }),
        );
    });

    it('keeps the current name when the replacement is blank', async () => {
        render(CalendarManager, { props: { calendars: [journal] } });

        await fireEvent.change(screen.getByLabelText('Name of Journal'), {
            target: { value: '   ' },
        });

        await Promise.resolve();
        expect(tauri.invoke.mock.calls.filter(([name]) => name === 'rename_calendar')).toHaveLength(
            0,
        );
    });

    it('deletes a list once the confirm is accepted', async () => {
        render(CalendarManager, { props: { calendars: [journal] } });

        await fireEvent.click(screen.getByLabelText('Delete Journal'));

        expect(window.confirm).toHaveBeenCalledWith(
            'Delete "Journal" and every events in it? This cannot be undone.',
        );
        await waitFor(() =>
            expect(invokedArgs('delete_calendar')).toEqual({ href: '/me/journal/' }),
        );
    });

    it('keeps the list when the confirm is declined', async () => {
        vi.mocked(window.confirm).mockReturnValue(false);
        render(CalendarManager, { props: { calendars: [journal] } });

        await fireEvent.click(screen.getByLabelText('Delete Journal'));

        await Promise.resolve();
        expect(tauri.invoke.mock.calls.filter(([name]) => name === 'delete_calendar')).toHaveLength(
            0,
        );
    });

    it('sends a shortened hex colour to the backend', async () => {
        render(CalendarManager, { props: { calendars: [journal] } });

        await fireEvent.change(screen.getByLabelText('Colour of Journal'), {
            target: { value: '#123456' },
        });

        await waitFor(() =>
            expect(invokedArgs('set_calendar_color')).toEqual({
                href: '/me/journal/',
                color: '#123456',
            }),
        );
    });
});
