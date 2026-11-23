import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/svelte';
import Harness from '../../test/FilterEditorHarness.svelte';
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
afterEach(() => { cleanup(); });

describe('nested filter editing', () => {
    it('builds and edits A OR (B AND C), preserving operators and sibling conditions', async () => {
        render(Harness, { props: { columns } });
        await fireEvent.click(screen.getByRole('button', { name: 'Filter' }));
        await fireEvent.change(screen.getByRole('combobox', { name: 'Group operator' }), { target: { value: 'or' } });
        await fireEvent.click(screen.getByRole('button', { name: /add filter/i }));
        await fireEvent.input(screen.getByRole('textbox', { name: 'Filter value' }), { target: { value: 'A' } });
        await fireEvent.click(screen.getByRole('button', { name: /add group/i }));

        const nested = within(screen.getByRole('group', { name: 'Filter group' }));
        await fireEvent.input(nested.getByRole('textbox', { name: 'Filter value' }), { target: { value: 'B' } });
        await fireEvent.click(nested.getByRole('button', { name: /add filter/i }));
        await fireEvent.input(nested.getAllByRole('textbox', { name: 'Filter value' })[1]!, { target: { value: 'C' } });
        expect(currentFilters()).toEqual({ operator: 'or', children: [leaf('A'), { kind: 'group', operator: 'and', children: [leaf('B'), leaf('C')] }] });
        expect(screen.getByRole('button', { name: 'Filter' }).textContent?.trim()).toBe('3');

        await fireEvent.change(nested.getByRole('combobox', { name: 'Group operator' }), { target: { value: 'or' } });
        await fireEvent.click(nested.getAllByRole('button', { name: 'Remove filter' })[0]!);
        expect(currentFilters()).toEqual({ operator: 'or', children: [leaf('A'), { kind: 'group', operator: 'or', children: [leaf('C')] }] });
        await fireEvent.click(nested.getByRole('button', { name: 'Remove filter group' }));
        expect(currentFilters()).toEqual({ operator: 'or', children: [leaf('A')] });
        expect(screen.getByRole('button', { name: 'Filter' }).textContent?.trim()).toBe('1');
    });

    it('supports deeper groups and clears the tree without resetting the root operator', async () => {
        render(Harness, { props: { columns } });
        await fireEvent.click(screen.getByRole('button', { name: 'Filter' }));
        await fireEvent.change(screen.getByRole('combobox', { name: 'Group operator' }), { target: { value: 'or' } });
        await fireEvent.click(screen.getByRole('button', { name: /add group/i }));
        const nested = within(screen.getByRole('group', { name: 'Filter group' }));
        await fireEvent.click(nested.getByRole('button', { name: /add group/i }));
        const deeper = within(screen.getAllByRole('group', { name: 'Filter group' })[1]!);
        await fireEvent.input(deeper.getByRole('textbox', { name: 'Filter value' }), { target: { value: 'Deep' } });
        expect(currentFilters()).toEqual({ operator: 'or', children: [{ kind: 'group', operator: 'and', children: [
            { kind: 'condition', filter: { column: 'title', operator: 'is', value: [] } },
            { kind: 'group', operator: 'and', children: [leaf('Deep')] },
        ] }] });
        await fireEvent.click(screen.getByRole('button', { name: 'Clear filters' }));
        expect(currentFilters()).toEqual({ operator: 'or', children: [] });
        expect(screen.queryByRole('group', { name: 'Filter group' })).toBeNull();
    });
});
