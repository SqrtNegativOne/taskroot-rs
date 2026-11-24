import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import EventFilterButton from './EventFilterButton.svelte';
import { mockTauriInvoke } from '../test/tauriMock';
import type { AppCalendar } from '../lib/domain';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

const calendars: AppCalendar[] = [
    { id: 'events', summary: 'Event Calendar', writable: true, component: 'VEVENT' },
    { id: 'tasks', summary: 'Task List', writable: true, component: 'VTODO' },
];

beforeEach(() => {
    tauri.reset();
    tauri.stubCommandValue('list_tags', ['home', 'work']);
});

afterEach(() => { cleanup(); });

describe('EventFilterButton', () => {
    it('keeps timeline filters calendar-only when no visibility choices are supplied', async () => {
        render(EventFilterButton, { props: { calendars } });
        await fireEvent.click(screen.getByTitle('Filter'));
        expect(screen.queryByRole('checkbox', { name: /hide/i })).toBeNull();
        await fireEvent.click(screen.getByRole('button', { name: /add filter/i }));
        await fireEvent.click(screen.getByRole('button', { name: /select\.\.\./i }));
        expect(screen.getByRole('checkbox', { name: 'Event Calendar' })).toBeTruthy();
        expect(screen.queryByRole('checkbox', { name: 'Task List' })).toBeNull();
    });

    it('offers existing tags as a multi-select that appends chips', async () => {
        render(EventFilterButton, { props: { calendars } });
        await fireEvent.click(screen.getByTitle('Filter'));
        await fireEvent.click(screen.getByRole('button', { name: /add filter/i }));

        const columnSelect = screen.getByRole('combobox', { name: 'Filter property' });
        await fireEvent.change(columnSelect, { target: { value: 'tags' } });

        await fireEvent.click(await screen.findByRole('button', { name: /select\.\.\./i }));
        await fireEvent.click(await screen.findByRole('checkbox', { name: 'work' }));
        await fireEvent.click(screen.getByRole('checkbox', { name: 'home' }));

        expect(screen.getByRole('button', { name: /work.*home/i })).toBeTruthy();
    });

    it('filters an optional enum by presence instead of a selected value', async () => {
        render(EventFilterButton, { props: { calendars } });
        await fireEvent.click(screen.getByRole('button', { name: 'Filter' }));
        await fireEvent.click(screen.getByRole('button', { name: /add filter/i }));
        await fireEvent.change(screen.getByRole('combobox', { name: 'Filter property' }), { target: { value: 'status' } });
        await fireEvent.click(screen.getByRole('button', { name: /select\.\.\./i }));
        await fireEvent.click(screen.getByRole('checkbox', { name: 'confirmed' }));
        await fireEvent.change(screen.getByRole('combobox', { name: 'Filter operator' }), { target: { value: 'is empty' } });
        expect(screen.queryByRole('button', { name: /confirmed/i })).toBeNull();
        expect(screen.queryByRole('checkbox', { name: 'confirmed' })).toBeNull();
        await fireEvent.change(screen.getByRole('combobox', { name: 'Filter operator' }), { target: { value: 'is' } });
        expect(screen.getByRole('button', { name: /select\.\.\./i })).toBeTruthy();
    });

    it('keeps the Filter button accessible by name when its badge is active', () => {
        render(EventFilterButton, { props: { calendars, visibility: { hideFrequentRecurring: true, hideTimed: true } } });
        expect(screen.getByRole('button', { name: 'Filter' })).toBeTruthy();
    });
});
