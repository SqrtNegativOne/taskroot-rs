import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import Harness from '../../test/FilterEditorHarness.svelte';
import { filterGroup } from '../../lib/domain';
import type { FilterColumn, FilterCondition } from '../../lib/domain';
import { mockTauriInvoke } from '../../test/tauriMock';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});
const tauri = mockTauriInvoke();
const columns: FilterColumn[] = [
    { id: 'start', label: 'Start', filterType: 'Date' },
    { id: 'due', label: 'Due', filterType: 'Date' },
    { id: 'tags', label: 'Tags' },
    { id: 'priority', label: 'Priority' },
    { id: 'status', label: 'Status' },
    { id: 'title', label: 'Title' },
];
const currentFilters = (): unknown => JSON.parse(screen.getByTestId('filters').textContent ?? 'null');

beforeEach(() => {
    tauri.reset();
    tauri.stubCommandValue('list_tags', ['work']);
});
async function openFilter(initial: FilterCondition) {
    render(Harness, { props: { columns, initial: filterGroup([initial]) } });
    await fireEvent.click(screen.getByRole('button', { name: 'Filter' }));
}

describe('empty-field filter editing', () => {
    it.each(['start', 'due', 'tags', 'priority', 'status'])('filters %s without a value control', async column => {
        await openFilter({ column, operator: 'is', value: ['stale'] });
        await fireEvent.change(screen.getByRole('combobox', { name: 'Filter operator' }), { target: { value: 'is empty' } });
        expect(currentFilters()).toEqual(filterGroup([{ column, operator: 'is empty', value: [] }]));
        expect(screen.queryByRole('textbox', { name: 'Filter value' })).toBeNull();
        expect(screen.queryByRole('button', { name: /^Select\.\.\./ })).toBeNull();

        await fireEvent.change(screen.getByRole('combobox', { name: 'Filter operator' }), { target: { value: 'is not empty' } });
        expect(currentFilters()).toEqual(filterGroup([{ column, operator: 'is not empty', value: [] }]));
        await fireEvent.change(screen.getByRole('combobox', { name: 'Filter operator' }), { target: { value: 'is' } });
        expect(currentFilters()).toEqual(filterGroup([{ column, operator: 'is', value: [] }]));
        if (column === 'tags') {
            await fireEvent.click(screen.getByRole('button', { name: /^Select\.\.\./ }));
            await fireEvent.click(await screen.findByRole('checkbox', { name: 'work' }));
        } else {
            await fireEvent.input(screen.getByRole('textbox', { name: 'Filter value' }), { target: { value: 'new' } });
        }
        expect(currentFilters()).toEqual(filterGroup([{ column, operator: 'is', value: [column === 'tags' ? 'work' : 'new'] }]));
    });

    it('restores an empty date condition and resets it when switching properties', async () => {
        await openFilter({ column: 'start', operator: 'is empty', value: [] });
        expect(screen.queryByRole('textbox', { name: 'Filter value' })).toBeNull();
        await fireEvent.change(screen.getByRole('combobox', { name: 'Filter property' }), { target: { value: 'title' } });
        expect(currentFilters()).toEqual(filterGroup([{ column: 'title', operator: 'is', value: [] }]));
        const value = screen.getByRole('textbox', { name: 'Filter value' });
        expect(value instanceof HTMLInputElement && value.value).toBe('');
    });

    it('offers date comparison operators and date placeholder only for date columns', async () => {
        const operatorValues = () => {
            const select = screen.getByRole('combobox', { name: 'Filter operator' });
            if (!(select instanceof HTMLSelectElement)) throw new Error('operator select missing');
            return Array.from(select.options).map((option) => option.value);
        };

        await openFilter({ column: 'start', operator: 'is', value: [] });
        expect(operatorValues()).toContain('before');
        expect(operatorValues()).toContain('on or after');
        expect(operatorValues()).toContain('is empty');
        await fireEvent.change(screen.getByRole('combobox', { name: 'Filter operator' }), { target: { value: 'before' } });
        expect(screen.getByPlaceholderText('YYYY-MM-DD or today')).toBeTruthy();

        await fireEvent.change(screen.getByRole('combobox', { name: 'Filter property' }), { target: { value: 'title' } });
        expect(operatorValues()).toContain('is');
        expect(operatorValues()).toContain('is not');
        expect(operatorValues()).not.toContain('before');
        expect(operatorValues()).not.toContain('after');
        expect(screen.getByPlaceholderText('Value...')).toBeTruthy();
    });
});
