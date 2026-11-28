import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import EventBlock from './EventBlock.svelte';
import { EVENT_INSET_PX, PIXELS_PER_HOUR, PX_PER_MIN } from './constants';
import { makeInstance } from '../../../test/factories';

describe('EventBlock geometry', () => {
    afterEach(cleanup);

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

    it('positions events relative to startMinute and visually clamps spanning pre-wake-up blocks', () => {
        const normal = render(EventBlock, {
            props: {
                event: makeInstance({ title: 'Standup' }),
                startMins: 9 * 60,
                endMins: 10 * 60,
                lane: 0,
                lanes: 1,
                startMinute: 6 * 60,
            },
        });
        const normalEl = normal.container.querySelector<HTMLElement>('.day-event')!;
        expect(Number.parseFloat(normalEl.style.top)).toBeCloseTo(
            3 * PIXELS_PER_HOUR + EVENT_INSET_PX,
            1,
        );
        expect(screen.getByText('09:00 – 10:00')).toBeTruthy();

        const spanning = render(EventBlock, {
            props: {
                event: makeInstance({ id: 'event-span', title: 'Early Shift' }),
                startMins: 5 * 60,
                endMins: 7 * 60,
                lane: 0,
                lanes: 1,
                startMinute: 6 * 60,
            },
        });
        const spanningEl = spanning.container.querySelector<HTMLElement>('.day-event')!;
        expect(Number.parseFloat(spanningEl.style.top)).toBeCloseTo(EVENT_INSET_PX, 1);
        expect(Number.parseFloat(spanningEl.style.height)).toBeCloseTo(
            PIXELS_PER_HOUR - 2 * EVENT_INSET_PX,
            1,
        );
        expect(screen.getByText('05:00 – 07:00')).toBeTruthy();
    });

    it('clamps upward move and top-edge resize at startMinute', async () => {
        const onMove = vi.fn();
        const onResize = vi.fn();
        const event = makeInstance();
        const { container } = render(EventBlock, {
            props: {
                event,
                startMins: 7 * 60,
                endMins: 8 * 60,
                lane: 0,
                lanes: 1,
                startMinute: 6 * 60,
                onMove,
                onResize,
            },
        });

        const block = container.querySelector<HTMLElement>('.day-event')!;
        await fireEvent.pointerDown(block, { button: 0, clientY: 200 });
        window.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientY: 20 }));
        window.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, clientY: 20 }));
        expect(onMove).toHaveBeenCalledWith(event, 6 * 60, 7 * 60);

        const topHandle = container.querySelector<HTMLElement>('.day-event-handle-top')!;
        await fireEvent.pointerDown(topHandle, { button: 0, clientY: 200 });
        window.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientY: 20 }));
        window.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, clientY: 20 }));
        expect(onResize).toHaveBeenCalledWith(event, 6 * 60, 8 * 60);
    });
});
