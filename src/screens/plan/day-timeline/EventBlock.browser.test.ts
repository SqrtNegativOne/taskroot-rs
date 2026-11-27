import { describe, expect, it } from 'vitest';
import { render } from '@testing-library/svelte';
import EventBlock from './EventBlock.svelte';
import { PX_PER_MIN } from './constants';
import { makeInstance } from '../../../test/factories';

describe('EventBlock geometry', () => {
    it('leaves a visible gap between back-to-back events', () => {
        const first = render(EventBlock, {
            props: { event: makeInstance(), startMins: 60, endMins: 120, lane: 0, lanes: 1 },
        });
        const second = render(EventBlock, {
            props: { event: makeInstance({ id: 'event-2' }), startMins: 120, endMins: 180, lane: 0, lanes: 1 },
        });
        const firstEl = first.container.querySelector<HTMLElement>('.day-event')!;
        const secondEl = second.container.querySelector<HTMLElement>('.day-event')!;

        const firstBottom = Number.parseFloat(firstEl.style.top) + Number.parseFloat(firstEl.style.height);
        const secondTop = Number.parseFloat(secondEl.style.top);

        expect(secondTop).toBeGreaterThan(firstBottom);
    });

    it('keeps a minimum visible height for very short events', () => {
        const { container } = render(EventBlock, {
            props: { event: makeInstance(), startMins: 60, endMins: 61, lane: 0, lanes: 1 },
        });
        const block = container.querySelector<HTMLElement>('.day-event')!;

        expect(Number.parseFloat(block.style.height)).toBeGreaterThan(PX_PER_MIN);
    });
});
