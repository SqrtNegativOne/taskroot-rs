import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import type { AppCalendar } from '../../lib/domain';
import InspectorTitle from './InspectorTitle.svelte';

const TASK_CALENDARS: AppCalendar[] = [
    { id: '/tasks/', summary: 'Tasks', writable: true, component: 'VTODO' },
    { id: '/work/', summary: 'Work', writable: true, component: 'VTODO' },
];

beforeEach(() => { vi.useFakeTimers(); });
afterEach(() => {
vi.useRealTimers();
});

describe('InspectorTitle', () => {
    it('keeps typing responsive and saves only the latest title after the debounce', async () => {
        const onchange = vi.fn();
        render(InspectorTitle, { value: 'Draft', onchange });
        const title = screen.getByRole('textbox', { name: 'Title' });
        await fireEvent.focus(title);

        await fireEvent.input(title, { target: { value: 'Draft proposal' } });
        await fireEvent.input(title, { target: { value: 'Final proposal' } });
        expect((title as HTMLTextAreaElement).value).toBe('Final proposal');
        vi.advanceTimersByTime(399);
        expect(onchange).not.toHaveBeenCalled();
        vi.advanceTimersByTime(1);
        expect(onchange).toHaveBeenCalledExactlyOnceWith('Final proposal');
    });

    it('saves immediately on blur without a duplicate delayed save', async () => {
        const onchange = vi.fn();
        render(InspectorTitle, { value: 'Draft', onchange });
        const title = screen.getByRole('textbox', { name: 'Title' });
        await fireEvent.focus(title);

        await fireEvent.input(title, { target: { value: 'Updated draft' } });
        await fireEvent.blur(title);

        expect(onchange).toHaveBeenCalledExactlyOnceWith('Updated draft');
        vi.advanceTimersByTime(400);
        expect(onchange).toHaveBeenCalledTimes(1);
    });

    it('saves a pending edit when the inspector closes', async () => {
        const onchange = vi.fn();
        const { unmount } = render(InspectorTitle, { value: 'Draft', onchange });
        const title = screen.getByRole('textbox', { name: 'Title' });
        await fireEvent.focus(title);
        await fireEvent.input(title, {
            target: { value: 'Revised draft' },
        });

        unmount();

        expect(onchange).toHaveBeenCalledExactlyOnceWith('Revised draft');
        vi.advanceTimersByTime(400);
        expect(onchange).toHaveBeenCalledTimes(1);
    });

    it('previews quick properties without premature debounced saves and commits clean title on Enter', async () => {
        vi.setSystemTime(new Date(2026, 10, 19, 10, 0, 0));
        const onchange = vi.fn();
        const onCommit = vi.fn();
        render(InspectorTitle, {
            value: 'Buy milk',
            onchange,
            quickEdit: { calendars: TASK_CALENDARS, onCommit },
        });
        const title = screen.getByRole('textbox', { name: 'Title' }) as HTMLTextAreaElement;
        await fireEvent.focus(title);
        await fireEvent.input(title, {
            target: { value: 'Buy milk tomorrow at 9:30am #errands p1 ^30m +work' },
        });

        expect(
            screen.getByText('Due Tomorrow 09:30 · #errands · P4 · 30 min · Work'),
        ).not.toBeNull();
        vi.advanceTimersByTime(500);
        expect(onchange).not.toHaveBeenCalled();
        expect(onCommit).not.toHaveBeenCalled();

        await fireEvent.keyDown(title, { key: 'Enter' });
        await fireEvent.blur(title);

        expect(onchange).not.toHaveBeenCalled();
        expect(onCommit).toHaveBeenCalledOnce();
        expect(onCommit.mock.calls[0]?.[0]).toMatchObject({
            title: 'Buy milk',
            due: '2026-11-20T09:30:00',
            hasTime: true,
            tags: ['errands'],
            priority: 4,
            estimateMinutes: 30,
            list: TASK_CALENDARS[1],
        });
        expect(title.value).toBe('Buy milk');
    });

    it('lets the user click Keep text to save a title containing a date or sigil literally', async () => {
        vi.setSystemTime(new Date(2026, 10, 19, 10, 0, 0));
        const onchange = vi.fn();
        const onCommit = vi.fn();
        render(InspectorTitle, {
            value: 'Retro',
            onchange,
            quickEdit: { calendars: TASK_CALENDARS, onCommit },
        });
        const title = screen.getByRole('textbox', { name: 'Title' });
        await fireEvent.focus(title);
        await fireEvent.input(title, { target: { value: 'Friday retrospective' } });

        const keepBtn = screen.getByRole('button', { name: 'Keep text' });
        await fireEvent.click(keepBtn);
        expect(screen.queryByRole('status')).toBeNull();

        await fireEvent.blur(title);

        expect(onCommit).not.toHaveBeenCalled();
        expect(onchange).toHaveBeenCalledExactlyOnceWith('Friday retrospective');
    });

    it('blocks Enter on an unknown list error and reports it on blur without saving broken tokens', async () => {
        const onchange = vi.fn();
        const onCommit = vi.fn();
        const onError = vi.fn();
        render(InspectorTitle, {
            value: 'Buy milk',
            onchange,
            quickEdit: { calendars: TASK_CALENDARS, onCommit, onError },
        });
        const title = screen.getByRole('textbox', { name: 'Title' }) as HTMLTextAreaElement;
        await fireEvent.focus(title);
        await fireEvent.input(title, { target: { value: 'Buy milk +missing' } });

        expect(screen.getByText('Unknown task list "missing"')).not.toBeNull();

        await fireEvent.keyDown(title, { key: 'Enter' });
        expect(onCommit).not.toHaveBeenCalled();
        expect(onError).not.toHaveBeenCalled();

        await fireEvent.blur(title);
        expect(onCommit).not.toHaveBeenCalled();
        expect(onchange).not.toHaveBeenCalled();
        expect(onError).toHaveBeenCalledExactlyOnceWith('Unknown task list "missing"');
        expect(title.value).toBe('Buy milk');
    });

    it('suppresses mid-typing debounce for a bare trailing sigil and still saves it on blur', async () => {
        const onchange = vi.fn();
        const onCommit = vi.fn();
        render(InspectorTitle, {
            value: 'Buy milk',
            onchange,
            quickEdit: { calendars: TASK_CALENDARS, onCommit },
        });
        const title = screen.getByRole('textbox', { name: 'Title' });
        await fireEvent.focus(title);
        await fireEvent.input(title, { target: { value: 'Buy milk #' } });

        vi.advanceTimersByTime(500);
        expect(onchange).not.toHaveBeenCalled();

        await fireEvent.blur(title);
        expect(onCommit).not.toHaveBeenCalled();
        expect(onchange).toHaveBeenCalledExactlyOnceWith('Buy milk #');
    });
});
