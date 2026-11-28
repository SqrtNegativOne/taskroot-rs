import { afterEach, describe, expect, it, vi } from 'vitest';
import {
    createSidebarWindowController,
    SIDEBAR_CLOSE_TRANSITION_MS,
    SIDEBAR_PANEL_WIDTH,
    SIDEBAR_TAB_HEIGHT,
    SIDEBAR_TAB_WIDTH,
    type SidebarMonitor,
    type SidebarWindowAdapter,
} from './state.svelte';

const MONITOR: SidebarMonitor = {
    scaleFactor: 2,
    position: { x: 0, y: 0 },
    size: { width: 3840, height: 2160 },
};

function makeAdapter(overrides: Partial<SidebarWindowAdapter> = {}) {
    const calls: string[] = [];
    const adapter: SidebarWindowAdapter = {
        show: vi.fn(() => {
            calls.push('show');
            return Promise.resolve();
        }),
        unminimize: vi.fn(() => {
            calls.push('unminimize');
            return Promise.resolve();
        }),
        setFocus: vi.fn(() => {
            calls.push('setFocus');
            return Promise.resolve();
        }),
        currentMonitor: vi.fn(() => Promise.resolve(MONITOR)),
        primaryMonitor: vi.fn(() => Promise.resolve(null)),
        setPosition: vi.fn((x, y) => {
            calls.push(`setPosition(${x},${y})`);
            return Promise.resolve();
        }),
        setSize: vi.fn((w, h) => {
            calls.push(`setSize(${w},${h})`);
            return Promise.resolve();
        }),
        ...overrides,
    };
    return { adapter, calls };
}

describe('createSidebarWindowController', () => {
    afterEach(() => {
        vi.useRealTimers();
    });

    it('cancels the pending 250ms close timer when reopened before the timer fires', async () => {
        vi.useFakeTimers();
        const { adapter, calls } = makeAdapter();
        const ctrl = createSidebarWindowController({ adapter });

        await ctrl.init();
        await ctrl.toggle(200);
        expect(ctrl.open).toBe(true);
        expect(ctrl.windowIsSmall).toBe(false);

        // Start closing: schedules 250ms window shrink timer.
        await ctrl.toggle(200);
        expect(ctrl.open).toBe(false);
        expect(ctrl.windowIsSmall).toBe(false);

        // Reopen within 250ms before the shrink timer fires.
        vi.advanceTimersByTime(100);
        calls.length = 0;
        await ctrl.toggle(200);
        expect(ctrl.open).toBe(true);
        expect(ctrl.windowIsSmall).toBe(false);

        const openWidth = (SIDEBAR_TAB_WIDTH + SIDEBAR_PANEL_WIDTH) * 2;
        expect(calls).toEqual([
            `setPosition(${3840 - openWidth},0)`,
            `setSize(${openWidth},2160)`,
        ]);

        // Advance past the original 250ms deadline: window must stay expanded.
        calls.length = 0;
        vi.advanceTimersByTime(SIDEBAR_CLOSE_TRANSITION_MS * 2);
        expect(ctrl.open).toBe(true);
        expect(ctrl.windowIsSmall).toBe(false);
        expect(calls).toEqual([]);
    });

    it('shrinks the window after 250ms when closing is not cancelled', async () => {
        vi.useFakeTimers();
        const { adapter, calls } = makeAdapter();
        const ctrl = createSidebarWindowController({ adapter });

        await ctrl.init();
        await ctrl.toggle(150);
        calls.length = 0;

        await ctrl.toggle(150);
        expect(ctrl.open).toBe(false);
        expect(ctrl.windowIsSmall).toBe(false);
        expect(calls).toEqual([]);

        await vi.advanceTimersByTimeAsync(SIDEBAR_CLOSE_TRANSITION_MS);
        const tabW = SIDEBAR_TAB_WIDTH * 2;
        const tabH = SIDEBAR_TAB_HEIGHT * 2;
        expect(ctrl.windowIsSmall).toBe(true);
        expect(calls).toEqual([
            `setPosition(${3840 - tabW},${150 * 2})`,
            `setSize(${tabW},${tabH})`,
        ]);
    });

    it('cancels the pending close timer on dispose so unmounting never shrinks a closed window', async () => {
        vi.useFakeTimers();
        const { adapter, calls } = makeAdapter();
        const ctrl = createSidebarWindowController({ adapter });

        await ctrl.init();
        await ctrl.toggle(150);
        calls.length = 0;

        await ctrl.toggle(150);
        ctrl.dispose();

        await vi.advanceTimersByTimeAsync(SIDEBAR_CLOSE_TRANSITION_MS * 2);
        expect(ctrl.windowIsSmall).toBe(false);
        expect(calls).toEqual([]);
    });

    it('ignores monitor resolution when disposed mid-init', async () => {
        const monitorDeferred = Promise.withResolvers<SidebarMonitor | null>();
        const primaryMonitor = vi.fn(() => Promise.resolve(MONITOR));
        const { adapter, calls } = makeAdapter({
            currentMonitor: () => monitorDeferred.promise,
            primaryMonitor,
        });
        const ctrl = createSidebarWindowController({ adapter });

        const initPromise = ctrl.init();
        await vi.waitFor(() => {
            expect(calls).toContain('setFocus');
        });

        ctrl.dispose();
        monitorDeferred.resolve(MONITOR);
        await initPromise;

        expect(ctrl.monitorReady).toBe(false);
        expect(primaryMonitor).not.toHaveBeenCalled();
    });

    it('aborts an in-flight expand when closed before setPosition resolves', async () => {
        vi.useFakeTimers();
        const setPosDeferred = Promise.withResolvers<void>();
        let posCount = 0;
        const { adapter, calls } = makeAdapter({
            setPosition: (x, y) => {
                posCount += 1;
                calls.push(`setPosition(${x},${y})`);
                return posCount === 1 ? setPosDeferred.promise : Promise.resolve();
            },
        });
        const ctrl = createSidebarWindowController({ adapter });

        await ctrl.init();
        calls.length = 0;

        const openPromise = ctrl.toggle(100);
        // Close again while expand's setPosition is still pending.
        const closePromise = ctrl.toggle(100);

        setPosDeferred.resolve();
        await Promise.all([openPromise, closePromise]);

        expect(ctrl.open).toBe(false);
        // Open's setSize must not have run after the close superseded it.
        expect(calls.filter((call) => call.startsWith('setSize'))).toEqual([]);

        await vi.advanceTimersByTimeAsync(SIDEBAR_CLOSE_TRANSITION_MS);
        expect(ctrl.windowIsSmall).toBe(true);
    });
});
