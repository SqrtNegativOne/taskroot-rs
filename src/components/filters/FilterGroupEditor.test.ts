import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/svelte';
import Harness from '../../test/FilterEditorHarness.svelte';
import { filterGroup } from '../../lib/domain';
import type { FilterCondition } from '../../lib/domain';
import { mockTauriInvoke } from '../../test/tauriMock';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});
const tauri = mockTauriInvoke();
const columns = [{ id: 'title', label: 'Title' }, { id: 'status', label: 'Status' }];
const leaf = (value: string) => ({ kind: 'condition', filter: { column: 'title', operator: 'is', value: [value] } });
const currentFilters = (): unknown => JSON.parse(screen.getByTestId('filters').textContent ?? 'null');

beforeEach(() => {
    tauri.reset();
    tauri.stubCommandValue('list_tags', []);
});
describe('inline filter group editing', () => {
    it('uses only the second rule to choose AND/OR for the whole group', async () => {
        render(Harness, { props: { columns } });
        await fireEvent.click(screen.getByRole('button', { name: 'Filter' }));
        expect(screen.queryByRole('combobox', { name: 'Group operator' })).toBeNull();
        expect(screen.queryByText('Match')).toBeNull();

        await fireEvent.click(screen.getByRole('button', { name: /add filter/i }));
        await fireEvent.input(screen.getByRole('textbox', { name: 'Filter value' }), { target: { value: 'A' } });
        expect(screen.getByText('Where')).toBeTruthy();
        expect(screen.queryByRole('combobox', { name: 'Group operator' })).toBeNull();

        await fireEvent.click(screen.getByRole('button', { name: /add filter/i }));
        await fireEvent.input(screen.getAllByRole('textbox', { name: 'Filter value' })[1]!, { target: { value: 'B' } });
        expect(currentFilters()).toEqual({ operator: 'and', children: [leaf('A'), leaf('B')] });
        await fireEvent.click(screen.getByRole('button', { name: /add filter/i }));
        await fireEvent.input(screen.getAllByRole('textbox', { name: 'Filter value' })[2]!, { target: { value: 'C' } });
        expect(screen.getAllByRole('combobox', { name: 'Group operator' })).toHaveLength(1);
        expect(screen.getByText('AND', { selector: 'span' })).toBeTruthy();

        await fireEvent.change(screen.getByRole('combobox', { name: 'Group operator' }), { target: { value: 'or' } });
        expect(currentFilters()).toEqual({ operator: 'or', children: [leaf('A'), leaf('B'), leaf('C')] });
        expect(screen.getByText('OR', { selector: 'span' })).toBeTruthy();
        await fireEvent.change(screen.getByRole('combobox', { name: 'Group operator' }), { target: { value: 'and' } });
        expect(currentFilters()).toEqual({ operator: 'and', children: [leaf('A'), leaf('B'), leaf('C')] });
    });

    it('builds and edits A OR (B AND C), preserving independent group operators', async () => {
        render(Harness, { props: { columns } });
        await fireEvent.click(screen.getByRole('button', { name: 'Filter' }));
        await fireEvent.click(screen.getByRole('button', { name: /add filter/i }));
        await fireEvent.input(screen.getByRole('textbox', { name: 'Filter value' }), { target: { value: 'A' } });
        await fireEvent.click(screen.getByRole('button', { name: /add group/i }));
        await fireEvent.change(screen.getByRole('combobox', { name: 'Group operator' }), { target: { value: 'or' } });

        const nested = within(screen.getByRole('group', { name: 'Filter group' }));
        expect(nested.queryByRole('combobox', { name: 'Group operator' })).toBeNull();
        await fireEvent.input(nested.getByRole('textbox', { name: 'Filter value' }), { target: { value: 'B' } });
        await fireEvent.click(nested.getByRole('button', { name: /add filter/i }));
        await fireEvent.input(nested.getAllByRole('textbox', { name: 'Filter value' })[1]!, { target: { value: 'C' } });
        expect(currentFilters()).toEqual({ operator: 'or', children: [leaf('A'), { kind: 'group', operator: 'and', children: [leaf('B'), leaf('C')] }] });
        expect(screen.getByRole('button', { name: 'Filter' }).textContent?.trim()).toBe('3');

        await fireEvent.change(nested.getByRole('combobox', { name: 'Group operator' }), { target: { value: 'or' } });
        await fireEvent.click(nested.getAllByRole('button', { name: 'Remove filter' })[0]!);
        expect(nested.queryByRole('combobox', { name: 'Group operator' })).toBeNull();
        expect(currentFilters()).toEqual({ operator: 'or', children: [leaf('A'), { kind: 'group', operator: 'or', children: [leaf('C')] }] });
        await fireEvent.click(nested.getByRole('button', { name: 'Remove filter group' }));
        expect(currentFilters()).toEqual({ operator: 'or', children: [leaf('A')] });
        expect(screen.queryByRole('combobox', { name: 'Group operator' })).toBeNull();
        expect(screen.getByRole('button', { name: 'Filter' }).textContent?.trim()).toBe('1');
    });

    it('supports deeper groups and clears the tree without resetting the root operator', async () => {
        render(Harness, { props: { columns, initial: filterGroup<FilterCondition>([{ column: 'title', operator: 'is', value: ['A'] }], 'or') } });
        await fireEvent.click(screen.getByRole('button', { name: 'Filter' }));
        await fireEvent.click(screen.getByRole('button', { name: /add group/i }));
        const nested = within(screen.getByRole('group', { name: 'Filter group' }));
        await fireEvent.click(nested.getByRole('button', { name: /add group/i }));
        const deeper = within(screen.getAllByRole('group', { name: 'Filter group' })[1]!);
        await fireEvent.input(deeper.getByRole('textbox', { name: 'Filter value' }), { target: { value: 'Deep' } });
        expect(currentFilters()).toEqual({ operator: 'or', children: [leaf('A'), { kind: 'group', operator: 'and', children: [
            { kind: 'condition', filter: { column: 'title', operator: 'is', value: [] } },
            { kind: 'group', operator: 'and', children: [leaf('Deep')] },
        ] }] });
        await fireEvent.click(screen.getByRole('button', { name: 'Clear filters' }));
        expect(currentFilters()).toEqual({ operator: 'or', children: [] });
        expect(screen.queryByRole('group', { name: 'Filter group' })).toBeNull();
        expect(screen.queryByRole('combobox', { name: 'Group operator' })).toBeNull();
    });

    it('keeps the operator when the second rule is removed and another rule takes its place', async () => {
        render(Harness, { props: { columns, initial: filterGroup<FilterCondition>([
            { column: 'title', operator: 'is', value: ['A'] },
            { column: 'title', operator: 'is', value: ['B'] },
            { column: 'title', operator: 'is', value: ['C'] },
        ], 'or') } });
        await fireEvent.click(screen.getByRole('button', { name: 'Filter' }));
        await fireEvent.click(screen.getAllByRole('button', { name: 'Remove filter' })[1]!);
        expect(currentFilters()).toEqual({ operator: 'or', children: [leaf('A'), leaf('C')] });
        expect(screen.getAllByRole('combobox', { name: 'Group operator' })).toHaveLength(1);
        await fireEvent.click(screen.getAllByRole('button', { name: 'Remove filter' })[0]!);
        expect(currentFilters()).toEqual({ operator: 'or', children: [leaf('C')] });
        expect(screen.queryByRole('combobox', { name: 'Group operator' })).toBeNull();
        await fireEvent.click(screen.getByRole('button', { name: /add filter/i }));
        const operator = screen.getByRole('combobox', { name: 'Group operator' });
        expect(operator instanceof HTMLSelectElement && operator.value).toBe('or');
    });
});
