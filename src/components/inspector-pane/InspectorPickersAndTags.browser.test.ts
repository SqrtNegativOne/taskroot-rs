import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke, type InvokeArgs } from '../../test/tauriMock';
import { makeEvent } from '../../test/factories';
import type { AppCalendar, AppTask } from '../../lib/domain';
import InspectorPane from './InspectorPane.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

function allInvokedArgs(command: string): Array<InvokeArgs | undefined> {
    return tauri.invoke.mock.calls
        .filter(([name]) => name === command)
        .map(([, args]) => args);
}

function getCalendarDay(ymd: string): HTMLElement {
    const day = document.querySelector<HTMLElement>(`[data-bits-day][data-value="${ymd}"]`);
    if (!day) throw new Error(`Expected calendar day ${ymd} to be rendered`);
    return day;
}

describe('InspectorPane date pickers', () => {
    beforeEach(() => {
        vi.useFakeTimers({ toFake: ['Date'] });
        vi.setSystemTime(new Date(2026, 10, 5, 10, 0, 0));
        tauri.reset();
        tauri.stubCommandValue('query_tasks', []);
        tauri.stubCommandValue('query_events', []);
        tauri.stubCommandValue('list_tags', []);
        tauri.stubCommandValue('get_active_calendars', []);
        tauri.stubCommandValue('get_ui_state', undefined);
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it('renders unified task date/time fields and edits date and time in the DatePicker popover', async () => {
        const task: AppTask = {
            id: 'task-1',
            title: 'Plan trip',
            due: '2026-11-08T17:00:00',
        };
        tauri.stubCommandValue('query_tasks', [task]);
        const onClose = vi.fn();

        render(InspectorPane, {
            props: {
                inspectorState: { type: 'task', id: task.id },
                onClose,
            },
        });

        const startTrigger = await screen.findByRole('button', { name: 'Start date' });
        const dueTrigger = screen.getByRole('button', { name: 'Due date' });
        expect(startTrigger.textContent?.trim()).toBe('No start date');
        expect(dueTrigger.textContent).toContain('· 17:00');

        await fireEvent.click(startTrigger);
        const day12 = getCalendarDay('2026-11-12');
        await fireEvent.pointerDown(day12);
        await fireEvent.click(day12);

        const timeInput = await screen.findByLabelText('Start time');
        await fireEvent.change(timeInput, { target: { value: '14:30' } });

        await waitFor(() => {
            expect(allInvokedArgs('update_task')).toEqual([
                { id: 'task-1', patches: [{ field: 'start', value: '2026-11-12' }] },
                { id: 'task-1', patches: [{ field: 'start', value: '2026-11-12T14:30:00' }] },
            ]);
        });
        expect(onClose).not.toHaveBeenCalled();

        await fireEvent.keyDown(document, { key: 'Escape' });
        await waitFor(() => {
            expect(document.querySelector('[data-inspector-popover]')).toBeNull();
        });
        expect(onClose).not.toHaveBeenCalled();

        await fireEvent.click(screen.getByRole('button', { name: 'Clear Due date' }));
        await waitFor(() => {
            expect(allInvokedArgs('update_task').at(-1)).toEqual({
                id: 'task-1',
                patches: [{ field: 'due', value: null }],
            });
        });
    });

    it('renders unified event schedule and edits range, time, and all-day in DateRangePicker popover', async () => {
        const event = makeEvent({
            id: 'event-1',
            title: 'Workshop',
            startTime: '2026-11-05T09:00:00',
            endTime: '2026-11-05T10:30:00',
            isAllDay: false,
        });
        tauri.stubCommandValue('query_events', [event]);
        const onClose = vi.fn();

        render(InspectorPane, {
            props: {
                inspectorState: { type: 'event', id: event.id },
                onClose,
            },
        });

        const whenTrigger = await screen.findByRole('button', { name: 'When' });
        expect(whenTrigger.textContent).toContain('· 09:00 – 10:30');
        expect(screen.queryByRole('switch', { name: 'All day event' })).toBeNull();

        await fireEvent.click(whenTrigger);

        const day10 = getCalendarDay('2026-11-10');
        const day12 = getCalendarDay('2026-11-12');
        await fireEvent.click(day10);
        await fireEvent.click(day12);

        await waitFor(() => {
            expect(allInvokedArgs('update_event').at(-1)).toEqual({
                id: 'event-1',
                patches: [
                    { field: 'startTime', value: '2026-11-10T09:00:00' },
                    { field: 'endTime', value: '2026-11-12T10:30:00' },
                    { field: 'isAllDay', value: false },
                ],
            });
        });

        await fireEvent.change(screen.getByLabelText('End time'), { target: { value: '17:00' } });
        await fireEvent.click(screen.getByRole('switch', { name: 'All day event' }));

        await waitFor(() => {
            expect(allInvokedArgs('update_event').slice(-2)).toEqual([
                {
                    id: 'event-1',
                    patches: [
                        { field: 'startTime', value: '2026-11-10T09:00:00' },
                        { field: 'endTime', value: '2026-11-12T17:00:00' },
                        { field: 'isAllDay', value: false },
                    ],
                },
                {
                    id: 'event-1',
                    patches: [
                        { field: 'startTime', value: '2026-11-10' },
                        { field: 'endTime', value: '2026-11-13' },
                        { field: 'isAllDay', value: true },
                    ],
                },
            ]);
        });

        const day15 = getCalendarDay('2026-11-15');
        await fireEvent.click(day15);
        await fireEvent.click(day15);

        await waitFor(() => {
            expect(allInvokedArgs('update_event').at(-1)).toEqual({
                id: 'event-1',
                patches: [
                    { field: 'startTime', value: '2026-11-15' },
                    { field: 'endTime', value: '2026-11-16' },
                    { field: 'isAllDay', value: true },
                ],
            });
        });
        expect(onClose).not.toHaveBeenCalled();
    });
});

describe('InspectorPane tag autocomplete and read-only states', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('query_tasks', []);
        tauri.stubCommandValue('query_events', []);
        tauri.stubCommandValue('get_active_calendars', []);
        tauri.stubCommandValue('get_ui_state', undefined);
    });

    it('shows prefix-prioritized unselected tag suggestions below the input and selects via click or keyboard', async () => {
        const task: AppTask = {
            id: 'task-1',
            title: 'Review PR',
            tags: [{ id: 't1', name: 'done' }],
        };
        tauri.stubCommandValue('query_tasks', [task]);
        tauri.stubCommandValue('list_tags', ['homework', 'work', 'workout', 'done']);
        const onClose = vi.fn();

        render(InspectorPane, {
            props: {
                inspectorState: { type: 'task', id: task.id },
                onClose,
            },
        });

        const tagInput = await screen.findByRole('combobox', { name: 'Tags' });
        await fireEvent.focus(tagInput);
        await fireEvent.input(tagInput, { target: { value: 'wor' } });

        const listbox = await screen.findByRole('listbox', { name: 'Tags suggestions' });
        const options = Array.from(listbox.querySelectorAll('[role="option"]')).map(el => el.textContent?.trim());
        expect(options).toEqual(['work', 'workout', 'homework']);

        await fireEvent.keyDown(tagInput, { key: 'ArrowDown' });
        await fireEvent.keyDown(tagInput, { key: 'Enter' });

        await waitFor(() => {
            expect(allInvokedArgs('update_task')).toEqual([
                {
                    id: 'task-1',
                    patches: [
                        {
                            field: 'tags',
                            value: [
                                expect.objectContaining({ name: 'done' }),
                                expect.objectContaining({ name: 'work' }),
                            ],
                        },
                    ],
                },
            ]);
        });

        await fireEvent.input(tagInput, { target: { value: 'home' } });
        const homeOption = await screen.findByRole('option', { name: 'homework' });
        await fireEvent.click(homeOption);

        await waitFor(() => {
            expect(allInvokedArgs('update_task')).toHaveLength(2);
        });

        await fireEvent.input(tagInput, { target: { value: 'w' } });
        expect(await screen.findByRole('listbox', { name: 'Tags suggestions' })).toBeTruthy();
        await fireEvent.keyDown(tagInput, { key: 'Escape' });
        expect(screen.queryByRole('listbox')).toBeNull();
        expect(onClose).not.toHaveBeenCalled();

        await fireEvent.input(tagInput, { target: { value: 'custom-tag' } });
        await fireEvent.keyDown(tagInput, { key: 'Enter' });
        await waitFor(() => {
            expect(allInvokedArgs('update_task').at(-1)).toEqual({
                id: 'task-1',
                patches: [
                    {
                        field: 'tags',
                        value: [
                            expect.objectContaining({ name: 'done' }),
                            expect.objectContaining({ name: 'custom-tag' }),
                        ],
                    },
                ],
            });
        });
    });

    it('offers tag suggestions for writable events', async () => {
        const calendars: AppCalendar[] = [
            { id: 'cal-rw', summary: 'Personal', writable: true, component: 'VEVENT' },
        ];
        const writableEvent = makeEvent({
            id: 'ev-rw',
            remoteCollectionId: 'cal-rw',
        });
        tauri.stubCommandValue('query_events', [writableEvent]);
        tauri.stubCommandValue('list_tags', ['planning', 'personal']);

        render(InspectorPane, {
            props: {
                inspectorState: { type: 'event', id: 'ev-rw' },
                calendars,
                onClose: vi.fn(),
            },
        });

        const tagInput = await screen.findByRole('combobox', { name: 'Tags' });
        await fireEvent.focus(tagInput);
        await fireEvent.input(tagInput, { target: { value: 'pl' } });
        await fireEvent.click(await screen.findByRole('option', { name: 'planning' }));

        await waitFor(() => {
            expect(allInvokedArgs('update_event')).toEqual([
                {
                    id: 'ev-rw',
                    patches: [
                        {
                            field: 'tags',
                            value: [expect.objectContaining({ name: 'planning' })],
                        },
                    ],
                },
            ]);
        });
    });

    it('disables tags input, remove buttons, and schedule picker in read-only calendars', async () => {
        const calendars: AppCalendar[] = [
            { id: 'cal-ro', summary: 'Holidays', writable: false, component: 'VEVENT' },
        ];
        const readOnlyEvent = makeEvent({
            id: 'ev-ro',
            remoteCollectionId: 'cal-ro',
            tags: [{ id: 't1', name: 'holiday' }],
        });
        tauri.stubCommandValue('query_events', [readOnlyEvent]);

        render(InspectorPane, {
            props: {
                inspectorState: { type: 'event', id: 'ev-ro' },
                calendars,
                onClose: vi.fn(),
            },
        });

        const roTagInput = (await screen.findByRole('combobox', { name: 'Tags' })) as HTMLInputElement;
        expect(roTagInput.disabled).toBe(true);
        const removeBtn = screen.getByRole('button', { name: 'Remove tag holiday' }) as HTMLButtonElement;
        expect(removeBtn.disabled).toBe(true);
        const roWhenBtn = screen.getByRole('button', { name: 'When' }) as HTMLButtonElement;
        expect(roWhenBtn.disabled).toBe(true);
    });
});
