import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/svelte';
import EventBlock from './EventBlock.svelte';
import { PX_PER_MIN } from './constants';
import { makeInstance } from '../../../test/factories';

describe('EventBlock geometry', () => {
    afterEach(() => {
        cleanup();
    });

    it('renders the event slightly inside its time slot so back-to-back events keep a gap', () => {
        const first = render(EventBlock, {
            props: { event: makeInstance(), startMins: 60, endMins: 120, lane: 0, lanes: 1 },
        });
        const second = render(EventBlock, {
            props: { event: makeInstance({ id: 'event-2' }), startMins: 120, endMins: 180, lane: 0, lanes: 1 },
        });
        const firstEl = first.container.querySelector<HTMLElement>('.day-event')!;
        const secondEl = second.container.querySelector<HTMLElement>('.day-event')!;

        const firstTop = Number.parseFloat(firstEl.style.top);
        const firstBottom = firstTop + Number.parseFloat(firstEl.style.height);
        const secondTop = Number.parseFloat(secondEl.style.top);

        expect(firstTop).toBeCloseTo(60 * PX_PER_MIN + 2, 5);
        expect(firstBottom).toBeCloseTo(120 * PX_PER_MIN - 2, 5);
        expect(secondTop - firstBottom).toBeCloseTo(4, 5);
    });

    it('keeps a minimum visible height for very short events', () => {
        const { container } = render(EventBlock, {
            props: { event: makeInstance(), startMins: 60, endMins: 61, lane: 0, lanes: 1 },
        });
        const block = container.querySelector<HTMLElement>('.day-event')!;

        expect(Number.parseFloat(block.style.height)).toBe(18);
        expect(Number.parseFloat(block.style.top)).toBeCloseTo(60 * PX_PER_MIN + 2, 5);
    });
});
