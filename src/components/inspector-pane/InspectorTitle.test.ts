import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import InspectorTitle from './InspectorTitle.svelte';

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
});
