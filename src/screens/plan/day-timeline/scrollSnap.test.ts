import { afterEach, describe, expect, it, vi } from 'vitest';
import { attachDaySnap, daySnapTarget, snapDayScroll } from './scrollSnap';

const IDLE_SNAP_DELAY_MS = 200;

describe('daySnapTarget', () => {
    it('keeps a position that already sits on a day boundary', () => {
        expect(daySnapTarget(200, [16, 216, 416], 500)).toBe(200);
    });

    it('aligns the day boundary nearest to where the scroll stopped', () => {
        expect(daySnapTarget(130, [16, 216, 416], 500)).toBe(200);
        expect(daySnapTarget(60, [16, 216, 416], 500)).toBe(0);
    });

    it('aligns the first day without scrolling past the start padding', () => {
        expect(daySnapTarget(0, [16, 216, 416], 500)).toBe(0);
    });

    it('clamps columns beyond the scroll range to the end of the range', () => {
        expect(daySnapTarget(232, [16, 216, 416], 232)).toBe(232);
        expect(daySnapTarget(150, [16, 216, 416], 100)).toBe(100);
    });

    it('settles at the start when the timeline cannot scroll sideways', () => {
        expect(daySnapTarget(0, [16, 216], 0)).toBe(0);
    });

    it('leaves the current offset alone when there are no day columns', () => {
        expect(daySnapTarget(42, [], 500)).toBe(42);
    });
});

function rect(left: number): DOMRect {
    return {
        left,
        right: left,
        top: 0,
        bottom: 0,
        width: 0,
        height: 0,
        x: left,
        y: 0,
        toJSON: () => ({}),
    };
}

interface ScrollerFixture {
    scroller: HTMLElement;
    columns: HTMLElement[];
}

function makeScroller(options: {
    columnLefts: number[];
    scrollLeft?: number;
    scrollWidth: number;
    clientWidth: number;
}): ScrollerFixture {
    const scroller = document.createElement('div');
    scroller.style.paddingLeft = '16px';
    const columns = options.columnLefts.map((left) => {
        const column = document.createElement('div');
        column.className = 'day-grid';
        column.getBoundingClientRect = () => rect(left);
        scroller.appendChild(column);
        return column;
    });
    scroller.getBoundingClientRect = () => rect(0);
    Object.defineProperty(scroller, 'scrollWidth', { value: options.scrollWidth, configurable: true });
    Object.defineProperty(scroller, 'clientWidth', { value: options.clientWidth, configurable: true });
    document.body.appendChild(scroller);
    scroller.scrollLeft = options.scrollLeft ?? 0;
    return { scroller, columns };
}

const overflowing = () =>
    makeScroller({ columnLefts: [16, 216, 416], scrollWidth: 1000, clientWidth: 500 });

describe('snapDayScroll', () => {
    afterEach(() => {
        document.body.innerHTML = '';
    });

    it('moves a sideways scroll that stopped between days onto the nearest boundary', () => {
        const { scroller } = overflowing();
        scroller.scrollLeft = 130;

        snapDayScroll(scroller);

        expect(scroller.scrollLeft).toBe(200);
    });

    it('does not move a scroll that is already aligned', () => {
        const { scroller } = overflowing();
        scroller.scrollLeft = 200;
        const setter = vi.fn();
        Object.defineProperty(scroller, 'scrollLeft', {
            get: () => 200,
            set: setter,
            configurable: true,
        });

        snapDayScroll(scroller);

        expect(setter).not.toHaveBeenCalled();
    });

    it('returns to the first day when the columns no longer overflow', () => {
        const { scroller } = makeScroller({
            columnLefts: [16, 216],
            scrollLeft: 40,
            scrollWidth: 468,
            clientWidth: 468,
        });

        snapDayScroll(scroller);

        expect(scroller.scrollLeft).toBe(0);
    });

    it('measures the boundaries again instead of assuming a fixed column width', () => {
        const { scroller, columns } = overflowing();
        scroller.scrollLeft = 450;
        for (const [index, column] of columns.entries()) {
            column.getBoundingClientRect = () => rect(16 + index * 240);
        }

        snapDayScroll(scroller);

        expect(scroller.scrollLeft).toBe(480);
    });
});

describe('attachDaySnap', () => {
    afterEach(() => {
        document.body.innerHTML = '';
        vi.useRealTimers();
    });

    it('snaps once the scroll reaches the end of the gesture', () => {
        const { scroller } = overflowing();

        const detach = attachDaySnap(scroller);
        scroller.scrollLeft = 130;
        scroller.dispatchEvent(new Event('scrollend'));

        expect(scroller.scrollLeft).toBe(200);
        detach();
    });

    it('stops snapping after it is detached', () => {
        const { scroller } = overflowing();

        const detach = attachDaySnap(scroller);
        detach();
        scroller.scrollLeft = 130;
        scroller.dispatchEvent(new Event('scrollend'));

        expect(scroller.scrollLeft).toBe(130);
    });

    describe('on engines without scrollend', () => {
        const saved = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'onscrollend');

        function removeScrollEnd(): void {
            Reflect.deleteProperty(HTMLElement.prototype, 'onscrollend');
        }

        function restoreScrollEnd(): void {
            if (saved) Object.defineProperty(HTMLElement.prototype, 'onscrollend', saved);
        }

        afterEach(restoreScrollEnd);

        it('snaps after the scroll has been idle briefly', () => {
            removeScrollEnd();
            vi.useFakeTimers();
            const { scroller } = overflowing();

            const detach = attachDaySnap(scroller);
            scroller.scrollLeft = 130;
            scroller.dispatchEvent(new Event('scroll'));
            vi.advanceTimersByTime(IDLE_SNAP_DELAY_MS - 1);
            expect(scroller.scrollLeft).toBe(130);

            vi.advanceTimersByTime(1);
            expect(scroller.scrollLeft).toBe(200);
            detach();
        });

        it('keeps waiting while the scroll keeps moving', () => {
            removeScrollEnd();
            vi.useFakeTimers();
            const { scroller } = overflowing();

            const detach = attachDaySnap(scroller);
            for (let i = 0; i < 3; i++) {
                scroller.scrollLeft = 40 + i * 30;
                scroller.dispatchEvent(new Event('scroll'));
                vi.advanceTimersByTime(IDLE_SNAP_DELAY_MS - 1);
            }
            expect(scroller.scrollLeft).toBe(100);

            vi.advanceTimersByTime(1);
            expect(scroller.scrollLeft).toBe(0);
            detach();
        });

        it('stops snapping after it is detached', () => {
            removeScrollEnd();
            vi.useFakeTimers();
            const { scroller } = overflowing();

            const detach = attachDaySnap(scroller);
            scroller.scrollLeft = 130;
            scroller.dispatchEvent(new Event('scroll'));
            detach();
            vi.advanceTimersByTime(IDLE_SNAP_DELAY_MS);

            expect(scroller.scrollLeft).toBe(130);
        });
    });
});
