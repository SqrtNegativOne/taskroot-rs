import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import FilterButton from './FilterButton.svelte';
import { mockTauriInvoke } from '../test/tauriMock';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

const columns = [
    { id: 'start', label: 'Start', filterType: 'Date' as const },
    { id: 'title', label: 'Title', filterType: 'Text' as const },
];

beforeEach(() => {
    tauri.reset();
    tauri.stubCommandValue('list_tags', []);
});

afterEach(() => { cleanup(); });

async function openWithFilter() {
    render(FilterButton, { props: { columns } });
    await fireEvent.click(screen.getByTitle('Filter'));
    await fireEvent.click(screen.getByRole('button', { name: /add filter/i }));
}

function operatorValues(): string[] {
    const select = screen.getAllByRole('combobox')[1];
    if (!(select instanceof HTMLSelectElement)) throw new Error('operator select missing');
    return Array.from(select.options).map((option) => option.value);
}

describe('FilterButton date columns', () => {
    it('offers comparison operators for a date column', async () => {
        await openWithFilter();

        expect(operatorValues()).toEqual([
            'is',
            'is not',
            'before',
            'after',
            'on or before',
            'on or after',
        ]);
    });

    it('falls back to equality operators for a text column', async () => {
        await openWithFilter();

        const columnSelect = screen.getAllByRole('combobox')[0];
        if (!(columnSelect instanceof HTMLSelectElement)) throw new Error('column select missing');
        await fireEvent.change(columnSelect, { target: { value: 'title' } });

        expect(operatorValues()).toEqual(['is', 'is not']);
    });
});
