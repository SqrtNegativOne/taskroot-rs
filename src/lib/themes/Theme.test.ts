import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, waitFor } from '@testing-library/svelte';
import Theme from './Theme.svelte';

afterEach(cleanup);

describe('application theme', () => {
    it('uses slate while settings are loading', () => {
        render(Theme);
        expect(document.documentElement.dataset.theme).toBe('slate');
    });

    it('updates the root selector when the saved setting changes', async () => {
        const view = render(Theme, { scheme: 'ember' });
        expect(document.documentElement.dataset.theme).toBe('ember');

        await view.rerender({ scheme: 'slate' });
        await waitFor(() => expect(document.documentElement.dataset.theme).toBe('slate'));
    });
});
