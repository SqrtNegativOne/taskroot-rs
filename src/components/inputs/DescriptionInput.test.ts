import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import DescriptionInput from './DescriptionInput.svelte';

function stubClipboard() {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
        value: { writeText },
        configurable: true,
        writable: true,
    });
    return writeText;
}

describe('DescriptionInput copy', () => {
    afterEach(() => {
        cleanup();
    });

    it('copies the description when the copy icon is pressed', async () => {
        const writeText = stubClipboard();
        render(DescriptionInput, {
            props: { value: 'Ship the release notes', onchange: vi.fn() },
        });

        await fireEvent.click(screen.getByRole('button', { name: 'Copy description' }));

        await waitFor(() => {
            expect(writeText).toHaveBeenCalledWith('Ship the release notes');
        });
    });

    it('flashes the icon to a check after copying', async () => {
        stubClipboard();
        render(DescriptionInput, {
            props: { value: 'Ship the release notes', onchange: vi.fn() },
        });

        await fireEvent.click(screen.getByRole('button', { name: 'Copy description' }));

        expect(await screen.findByRole('button', { name: 'Copied' })).toBeTruthy();
    });

    it('reverts the icon back to copy after the flash', async () => {
        vi.useFakeTimers();
        try {
            stubClipboard();
            render(DescriptionInput, {
                props: { value: 'Ship the release notes', onchange: vi.fn() },
            });

            await fireEvent.click(screen.getByRole('button', { name: 'Copy description' }));
            expect(screen.getByRole('button', { name: 'Copied' })).toBeTruthy();

            await vi.advanceTimersByTimeAsync(1500);
            expect(screen.queryByRole('button', { name: 'Copied' })).toBeNull();
            expect(screen.getByRole('button', { name: 'Copy description' })).toBeTruthy();
        } finally {
            vi.useRealTimers();
        }
    });

    it('leaves the icon alone when the clipboard write fails', async () => {
        const writeText = vi.fn().mockRejectedValue(new Error('denied'));
        Object.defineProperty(navigator, 'clipboard', {
            value: { writeText },
            configurable: true,
            writable: true,
        });
        render(DescriptionInput, {
            props: { value: 'Ship the release notes', onchange: vi.fn() },
        });

        await fireEvent.click(screen.getByRole('button', { name: 'Copy description' }));

        await waitFor(() => {
            expect(writeText).toHaveBeenCalledOnce();
        });
        expect(screen.queryByRole('button', { name: 'Copied' })).toBeNull();
    });

    it('hides the copy button when there is no description', () => {
        stubClipboard();
        render(DescriptionInput, { props: { value: undefined, onchange: vi.fn() } });

        expect(screen.queryByRole('button', { name: /copy description/i })).toBeNull();
    });

    it('hides the copy button while the description is only whitespace', () => {
        stubClipboard();
        render(DescriptionInput, { props: { value: '   ', onchange: vi.fn() } });

        expect(screen.queryByRole('button', { name: /copy description/i })).toBeNull();
    });

    it('hides the copy button while editing', async () => {
        stubClipboard();
        render(DescriptionInput, {
            props: { value: 'Ship the release notes', onchange: vi.fn() },
        });

        await fireEvent.click(screen.getByRole('button', { name: 'Description' }));

        expect(await screen.findByRole('textbox')).toBeTruthy();
        expect(screen.queryByRole('button', { name: /copy description/i })).toBeNull();
    });
});
