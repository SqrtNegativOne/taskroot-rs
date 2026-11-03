import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import EventFilterButton from './EventFilterButton.svelte';
import type { AppCalendar } from '../lib/domain';

const calendars: AppCalendar[] = [
    { id: 'events', summary: 'Event Calendar', writable: true, component: 'VEVENT' },
    { id: 'tasks', summary: 'Task List', writable: true, component: 'VTODO' },
];

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

    it('keeps the Filter button accessible by name when its badge is active', () => {
        render(EventFilterButton, { props: { calendars, visibility: { hideRecurring: true, hideTimed: true } } });
        expect(screen.getByRole('button', { name: 'Filter' })).toBeTruthy();
    });
});
