import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/svelte';
import SettingRow from './SettingRow.svelte';
import type { AppCalendar, SettingMeta, SettingValue } from '$lib/domain';

const numericSelect: SettingMeta = {
    id: 'default_task_duration',
    label: 'Default Duration',
    keywords: [],
    type: 'select',
    defaultValue: 0,
    options: [
        { value: 0, label: 'Not set' },
        { value: 15, label: '15m' },
    ],
};

const stringSelect: SettingMeta = {
    id: 'clock_style',
    label: 'Clock Style',
    keywords: [],
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
                    keywords: [],
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
                    keywords: [],
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
                    keywords: [],
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

describe('SettingRow time input', () => {
    afterEach(cleanup);

    const wakeUpSetting: SettingMeta = {
        id: 'earliest_wake_up_time',
        label: 'Earliest Wake Up Time',
        keywords: [],
        type: 'time',
        defaultValue: 0,
    };

    it('displays HH:MM for stored minutes and emits integer minutes on change', async () => {
        let emitted: SettingValue | undefined;
        render(SettingRow, {
            props: {
                item: wakeUpSetting,
                value: 360,
                onchange: (next: SettingValue) => {
                    emitted = next;
                },
            },
        });

        const input = screen.getByLabelText('Earliest Wake Up Time') as HTMLInputElement;
        expect(input.type).toBe('time');
        expect(input.value).toBe('06:00');

        await fireEvent.change(input, { target: { value: '06:30' } });
        expect(emitted).toBe(390);

        await fireEvent.input(input, { target: { value: '00:00' } });
        expect(emitted).toBe(0);
    });

    it('falls back to 00:00 when unset and ignores empty partial values', async () => {
        const emissions: SettingValue[] = [];
        render(SettingRow, {
            props: {
                item: wakeUpSetting,
                value: undefined,
                onchange: (next: SettingValue) => {
                    emissions.push(next);
                },
            },
        });

        const input = screen.getByLabelText('Earliest Wake Up Time') as HTMLInputElement;
        expect(input.value).toBe('00:00');

        await fireEvent.input(input, { target: { value: '' } });
        expect(emissions).toEqual([]);
    });
});
