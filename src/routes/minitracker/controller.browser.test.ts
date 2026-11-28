import { describe, expect, it, vi } from 'vitest';
import {
    createMinitrackerController,
    type MonitorWorkArea,
    type StoredPosition,
    type WindowGeometryAdapter,
} from './controller.svelte';

const BAR_SIZE = { width: 220, height: 48 };
const MONITOR: MonitorWorkArea = {
    scaleFactor: 1,
    workArea: { x: 0, y: 0, width: 1920, height: 1040 },
};

function makeDragElement(): HTMLDivElement {
    const el = document.createElement('div');
    el.setPointerCapture = vi.fn();
    el.releasePointerCapture = vi.fn();
    return el;
}

function makePointerEvent(
    el: HTMLElement,
    overrides: { button?: number; pointerId?: number; screenX?: number; screenY?: number } = {},
): PointerEvent {
    const event = new PointerEvent('pointerdown', {
        button: overrides.button ?? 0,
        pointerId: overrides.pointerId ?? 1,
        screenX: overrides.screenX ?? 100,
        screenY: overrides.screenY ?? 200,
    });
    Object.defineProperty(event, 'currentTarget', { value: el });
    Object.defineProperty(event, 'target', { value: el });
    return event;
}

describe('createMinitrackerController transitions', () => {
    it('aborts an in-flight openMenu when pointerDown starts before openMenu mutates the window', async () => {
        const firstOuterPos = Promise.withResolvers<{ x: number; y: number }>();
        const calls: string[] = [];
        let outerPosCalls = 0;

        const geometry: WindowGeometryAdapter = {
            outerPosition: () => {
                outerPosCalls += 1;
                if (outerPosCalls === 1) return firstOuterPos.promise;
                return Promise.resolve({ x: 300, y: 120 });
            },
            outerSize: () => Promise.resolve({ width: BAR_SIZE.width, height: BAR_SIZE.height }),
            scaleFactor: () => Promise.resolve(1),
            currentMonitor: () => Promise.resolve(MONITOR),
            setSize: (w, h) => {
                calls.push(`setSize(${w},${h})`);
                return Promise.resolve();
            },
            setPosition: (x, y) => {
                calls.push(`setPosition(${x},${y})`);
                return Promise.resolve();
            },
        };

        const saved: StoredPosition[] = [];
        const controller = createMinitrackerController({
            savePosition: (pos) => saved.push(pos),
            geometry,
        });

        const openPromise = controller.toggleMenu(4, BAR_SIZE);

        const el = makeDragElement();
        const dragPromise = controller.pointerDown(makePointerEvent(el, { screenX: 100, screenY: 100 }));

        // Release the stale openMenu geometry read after pointerDown has claimed the transition.
        firstOuterPos.resolve({ x: 300, y: 120 });
        await Promise.all([openPromise, dragPromise]);

        expect(controller.menuOpen).toBe(false);
        expect(calls).toEqual([]);

        controller.pointerMove(makePointerEvent(el, { screenX: 150, screenY: 160 }));
        controller.pointerUp(makePointerEvent(el, { screenX: 150, screenY: 160 }));

        expect(saved).toEqual([{ x: 350, y: 180 }]);
        expect(calls).toEqual(['setPosition(350,180)']);
    });

    it('serializes pointerDown behind an openMenu already resizing the window, then closes cleanly before dragging', async () => {
        const openResize = Promise.withResolvers<void>();
        const calls: string[] = [];
        let sizeCalls = 0;

        const geometry: WindowGeometryAdapter = {
            outerPosition: () => Promise.resolve({ x: 300, y: 120 }),
            outerSize: () => Promise.resolve({ width: BAR_SIZE.width, height: BAR_SIZE.height }),
            scaleFactor: () => Promise.resolve(1),
            currentMonitor: () => Promise.resolve(MONITOR),
            setSize: (w, h) => {
                sizeCalls += 1;
                calls.push(`setSize(${w},${h})`);
                return sizeCalls === 1 ? openResize.promise : Promise.resolve();
            },
            setPosition: (x, y) => {
                calls.push(`setPosition(${x},${y})`);
                return Promise.resolve();
            },
        };

        const saved: StoredPosition[] = [];
        const controller = createMinitrackerController({
            savePosition: (pos) => saved.push(pos),
            geometry,
        });

        const openPromise = controller.toggleMenu(3, BAR_SIZE);
        await vi.waitFor(() => {
            expect(calls).toHaveLength(1);
        });
        expect(controller.menuOpen).toBe(true);

        const el = makeDragElement();
        const dragPromise = controller.pointerDown(makePointerEvent(el, { screenX: 100, screenY: 100 }));

        // pointerDown must not close or move the window until openMenu's resize/position completes.
        await Promise.resolve();
        expect(calls).toHaveLength(1);

        openResize.resolve();
        await Promise.all([openPromise, dragPromise]);

        expect(controller.menuOpen).toBe(false);
        expect(calls).toEqual([
            'setSize(220,202)',
            'setPosition(300,120)',
            'setSize(220,48)',
            'setPosition(300,120)',
        ]);

        controller.pointerMove(makePointerEvent(el, { screenX: 140, screenY: 130 }));
        controller.pointerUp(makePointerEvent(el, { screenX: 140, screenY: 130 }));
        expect(saved).toEqual([{ x: 340, y: 150 }]);
    });

    it('cancels an in-flight openMenu when toggled closed before geometry resolves', async () => {
        const pendingPos = Promise.withResolvers<{ x: number; y: number }>();
        const setSize = vi.fn(() => Promise.resolve());
        const setPosition = vi.fn(() => Promise.resolve());

        const geometry: WindowGeometryAdapter = {
            outerPosition: () => pendingPos.promise,
            outerSize: () => Promise.resolve(BAR_SIZE),
            scaleFactor: () => Promise.resolve(1),
            currentMonitor: () => Promise.resolve(MONITOR),
            setSize,
            setPosition,
        };

        const controller = createMinitrackerController({
            savePosition: vi.fn(),
            geometry,
        });

        const first = controller.toggleMenu(3, BAR_SIZE);
        const second = controller.toggleMenu(3, BAR_SIZE);

        pendingPos.resolve({ x: 300, y: 120 });
        await Promise.all([first, second]);

        expect(controller.menuOpen).toBe(false);
        expect(setSize).not.toHaveBeenCalled();
        expect(setPosition).not.toHaveBeenCalled();
    });

    it('does not become draggable or save position if pointerUp fires before drag geometry resolves', async () => {
        const pendingPos = Promise.withResolvers<{ x: number; y: number }>();
        const setPosition = vi.fn(() => Promise.resolve());
        const savePosition = vi.fn();

        const geometry: WindowGeometryAdapter = {
            outerPosition: () => pendingPos.promise,
            outerSize: () => Promise.resolve(BAR_SIZE),
            scaleFactor: () => Promise.resolve(1),
            currentMonitor: () => Promise.resolve(MONITOR),
            setSize: () => Promise.resolve(),
            setPosition,
        };

        const controller = createMinitrackerController({ savePosition, geometry });
        const el = makeDragElement();

        const downPromise = controller.pointerDown(makePointerEvent(el, { screenX: 100, screenY: 100 }));
        controller.pointerUp(makePointerEvent(el, { screenX: 100, screenY: 100 }));

        pendingPos.resolve({ x: 300, y: 120 });
        await downPromise;

        controller.pointerMove(makePointerEvent(el, { screenX: 200, screenY: 200 }));
        expect(controller.isClickDimmed).toBe(false);
        expect(setPosition).not.toHaveBeenCalled();
        expect(savePosition).not.toHaveBeenCalled();
    });

    it('snaps dragged positions within SNAP_THRESHOLD of monitor workArea edges', async () => {
        const saved: StoredPosition[] = [];
        const setPosition = vi.fn(() => Promise.resolve());

        const geometry: WindowGeometryAdapter = {
            outerPosition: () => Promise.resolve({ x: 50, y: 50 }),
            outerSize: () => Promise.resolve(BAR_SIZE),
            scaleFactor: () => Promise.resolve(1),
            currentMonitor: () => Promise.resolve(MONITOR),
            setSize: () => Promise.resolve(),
            setPosition,
        };

        const controller = createMinitrackerController({
            savePosition: (pos) => saved.push(pos),
            geometry,
        });

        const el = makeDragElement();
        await controller.pointerDown(makePointerEvent(el, { screenX: 100, screenY: 100 }));
        // Move to (10, 15) -> both within 24px of workArea (0, 0)
        controller.pointerMove(makePointerEvent(el, { screenX: 60, screenY: 65 }));
        controller.pointerUp(makePointerEvent(el, { screenX: 60, screenY: 65 }));

        expect(setPosition).toHaveBeenCalledWith(0, 0);
        expect(saved).toEqual([{ x: 0, y: 0 }]);
    });
});
