import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/svelte';
import SettingRow from './SettingRow.svelte';
import type { SettingValue, SettingsSchemaItem } from './schema';
import type { AppCalendar } from '../../lib/domain';

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
    defaultValue: 'flowtime',
    options: [
        { value: 'flowtime', label: 'Flowtime' },
        { value: 'counter', label: 'Counter' },
    ],
};

const calendars: AppCalendar[] = [
    {
        id: '/me/chores/',
        summary: 'Chores',
        writable: true,
        component: 'VTODO',
    },
    {
        id: '/me/journal/',
        summary: 'Journal',
        writable: true,
        component: 'VEVENT',
    },
    {
        id: '/me/archive/',
        summary: 'Archived tasks',
        writable: false,
        component: 'VTODO',
    },
];

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
                value: 'flowtime',
                onchange: (next: SettingValue) => {
                    emitted = next;
                },
            },
        });

        await fireEvent.change(screen.getByRole('combobox'), { target: { value: 'counter' } });

        expect(emitted).toBe('counter');
    });
});

describe('SettingRow calendar select', () => {
    afterEach(cleanup);

    it('lists only writable task lists for default_task_list and emits the href', async () => {
        let emitted: SettingValue | undefined;
        render(SettingRow, {
            props: {
                item: {
                    id: 'default_task_list',
                    label: 'Default Tasks List',
                    type: 'calendar',
                    defaultValue: '',
                },
                value: '',
                calendars,
                onchange: (next: SettingValue) => {
                    emitted = next;
                },
            },
        });

        const select = screen.getByRole('combobox');
        expect(within(select).getByRole('option', { name: 'First writable list' })).not.toBeNull();
        expect(within(select).getByRole('option', { name: 'Chores' })).not.toBeNull();
        expect(within(select).queryByRole('option', { name: 'Journal' })).toBeNull();
        expect(within(select).queryByRole('option', { name: 'Archived tasks' })).toBeNull();

        await fireEvent.change(select, { target: { value: '/me/chores/' } });

        expect(emitted).toBe('/me/chores/');
    });

    it('lists only writable event lists for default_event_list', async () => {
        let emitted: SettingValue | undefined;
        render(SettingRow, {
            props: {
                item: {
                    id: 'default_event_list',
                    label: 'Default Events List',
                    type: 'calendar',
                    defaultValue: '',
                },
                value: '',
                calendars,
                onchange: (next: SettingValue) => {
                    emitted = next;
                },
            },
        });

        const select = screen.getByRole('combobox');
        expect(within(select).getByRole('option', { name: 'Journal' })).not.toBeNull();
        expect(within(select).queryByRole('option', { name: 'Chores' })).toBeNull();

        await fireEvent.change(select, { target: { value: '/me/journal/' } });

        expect(emitted).toBe('/me/journal/');
    });

    it('keeps a stored value that no longer matches a writable list', () => {
        render(SettingRow, {
            props: {
                item: {
                    id: 'default_task_list',
                    label: 'Default Tasks List',
                    type: 'calendar',
                    defaultValue: '',
                },
                value: '/me/gone/',
                calendars,
                onchange: () => { /* noop */ },
            },
        });

        expect(screen.getByRole('option', { name: '(missing list) /me/gone/' })).not.toBeNull();
        expect((screen.getByRole('combobox') as HTMLSelectElement).value).toBe('/me/gone/');
    });
});
