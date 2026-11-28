import { createRawSnippet } from 'svelte';
import { describe, expect, it } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import SplitPane from './SplitPane.svelte';

function textSnippet(text: string) {
    return createRawSnippet(() => ({
        render: () => `<div>${text}</div>`,
    }));
}

describe('SplitPane', () => {
    it('ends dragging and restores pointer events when pointercancel fires', async () => {
        const { container } = render(SplitPane, {
            props: {
                direction: 'horizontal',
                defaultSize: 200,
                pane1: textSnippet('Left'),
                pane2: textSnippet('Right'),
            },
        });

        const root = container.querySelector('.split-pane-container') as HTMLElement;
        const divider = screen.getByRole('separator');

        await fireEvent.pointerDown(divider, { pointerId: 1, clientX: 200 });
        expect(divider.classList.contains('dragging')).toBe(true);
        expect(root.style.pointerEvents).toBe('none');

        await fireEvent.pointerCancel(divider, { pointerId: 1 });
        expect(divider.classList.contains('dragging')).toBe(false);
        expect(root.style.pointerEvents).toBe('auto');
    });
});
