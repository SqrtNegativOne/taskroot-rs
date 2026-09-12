import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import SettingRow from './SettingRow.svelte';
import type { SettingValue, SettingsSchemaItem } from './schema';

const numericSelect: SettingsSchemaItem = {
    id: 'default_task_duration',
    label: 'Default Duration',
    type: 'select',
    defaultValue: 0,
    options: [
        { value: 0, label: 'Not set' },
        { value: 15, label: '15m' },
    ],
};

const stringSelect: SettingsSchemaItem = {
    id: 'clock_style',
    label: 'Clock Style',
    type: 'select',
    defaultValue: 'guzey',
    options: [
        { value: 'counter', label: 'Counter' },
        { value: 'guzey', label: 'Guzey' },
    ],
};

describe('SettingRow select', () => {
    afterEach(cleanup);

    it('emits a number for a numeric option instead of the raw DOM string', async () => {
        let emitted: SettingValue | undefined;
        render(SettingRow, {
            props: {
                item: numericSelect,
                value: 0,
                onchange: (next: SettingValue) => {
                    emitted = next;
                },
            },
        });

        await fireEvent.change(screen.getByRole('combobox'), { target: { value: '15' } });

        expect(emitted).toBe(15);
    });

    it('emits the string value for a string option', async () => {
        let emitted: SettingValue | undefined;
        render(SettingRow, {
            props: {
                item: stringSelect,
                value: 'guzey',
                onchange: (next: SettingValue) => {
                    emitted = next;
                },
            },
        });

        await fireEvent.change(screen.getByRole('combobox'), { target: { value: 'counter' } });

        expect(emitted).toBe('counter');
    });
});
