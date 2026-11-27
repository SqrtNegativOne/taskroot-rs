import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
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

async function openWithFilter() {
    render(FilterButton, { props: { columns } });
    await fireEvent.click(screen.getByTitle('Filter'));
    await fireEvent.click(screen.getByRole('button', { name: /add filter/i }));
}

function operatorValues(): string[] {
    const select = screen.getByRole('combobox', { name: 'Filter operator' });
    if (!(select instanceof HTMLSelectElement)) throw new Error('operator select missing');
    return Array.from(select.options).map((option) => option.value);
}

describe('FilterButton date columns', () => {
    it('offers comparison operators for a date column', async () => {
        await openWithFilter();

        expect(operatorValues()).toContain('before');
        expect(operatorValues()).toContain('on or after');
        expect(operatorValues()).toContain('is empty');
        await fireEvent.change(screen.getByRole('combobox', { name: 'Filter operator' }), { target: { value: 'before' } });
        expect(screen.getByPlaceholderText('YYYY-MM-DD or today')).toBeTruthy();
    });

    it('does not offer date comparisons for a text column', async () => {
        await openWithFilter();

        const columnSelect = screen.getByRole('combobox', { name: 'Filter property' });
        if (!(columnSelect instanceof HTMLSelectElement)) throw new Error('column select missing');
        await fireEvent.change(columnSelect, { target: { value: 'title' } });

        expect(operatorValues()).toContain('is');
        expect(operatorValues()).toContain('is not');
        expect(operatorValues()).not.toContain('before');
        expect(operatorValues()).not.toContain('after');
        expect(screen.getByPlaceholderText('Value...')).toBeTruthy();
    });
});
