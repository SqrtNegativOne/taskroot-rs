import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { mockTauriInvoke } from '../test/tauriMock';
import { makeEvent } from '../test/factories';
import { clickOutside } from './clickOutside';
import MultiSelect from '../components/inputs/MultiSelect.svelte';
import SelectInput from '../components/inputs/SelectInput.svelte';
import InspectorPane from '../components/inspector-pane/InspectorPane.svelte';
import MoreScreensDropdown from '../components/shell/MoreScreensDropdown.svelte';
import TaskContextMenu from '../components/tasklist/TaskContextMenu.svelte';
import CalendarHeader from '../screens/plan/date-grid/CalendarHeader.svelte';
import { DateGridView } from '../screens/plan/date-grid/constants';
import TimelineControls from '../screens/plan/day-timeline/components/TimelineControls.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

vi.mock('$app/navigation', () => ({
    goto: vi.fn(() => Promise.resolve()),
}));

vi.mock('$app/paths', () => ({
    resolve: (path: string) => path,
}));

const tauri = mockTauriInvoke();

interface ListenerTracker {
    activeCount: (eventType: string) => number;
    addedCount: (eventType: string) => number;
    restore: () => void;
}

function trackListeners(target: Document | Window): ListenerTracker {
    const activeByType = new Map<string, Set<EventListenerOrEventListenerObject>>();
    const addedByType = new Map<string, number>();

    const origAdd = target.addEventListener.bind(target);
    const origRemove = target.removeEventListener.bind(target);

    const addSpy = vi.spyOn(target, 'addEventListener').mockImplementation(
        (type, listener, options) => {
            if (listener) {
                let set = activeByType.get(type);
                if (!set) {
                    set = new Set();
                    activeByType.set(type, set);
                }
                set.add(listener);
                addedByType.set(type, (addedByType.get(type) ?? 0) + 1);
            }
            origAdd(type, listener, options);
        },
    );

    const removeSpy = vi.spyOn(target, 'removeEventListener').mockImplementation(
        (type, listener, options) => {
            if (listener) {
                activeByType.get(type)?.delete(listener);
            }
            origRemove(type, listener, options);
        },
    );

    return {
        activeCount(eventType: string) { return activeByType.get(eventType)?.size ?? 0; },
        addedCount(eventType: string) { return addedByType.get(eventType) ?? 0; },
        restore() {
            addSpy.mockRestore();
            removeSpy.mockRestore();
        },
    };
}

describe('clickOutside attachment', () => {
    let docListeners: ListenerTracker;
    let winListeners: ListenerTracker;

    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('query_tasks', []);
        tauri.stubCommandValue('query_events', []);
        tauri.stubCommandValue('get_active_calendars', []);
        tauri.stubCommandValue('list_tags', []);
        tauri.stubCommandValue('get_ui_state', undefined);
        docListeners = trackListeners(document);
        winListeners = trackListeners(window);
    });

    afterEach(() => {
        cleanup();
        docListeners.restore();
        winListeners.restore();
        vi.useRealTimers();
    });

    it('registers no global pointerdown listener while disabled and cleans up when detached', () => {
        const host = document.createElement('div');
        const inner = document.createElement('button');
        host.appendChild(inner);
        document.body.appendChild(host);

        const onOutside = vi.fn();
        try {
            const disabledCleanup = clickOutside(onOutside, { enabled: false })(host);
            expect(disabledCleanup).toBeUndefined();
            expect(docListeners.addedCount('pointerdown')).toBe(0);
            expect(docListeners.activeCount('pointerdown')).toBe(0);

            document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
            expect(onOutside).not.toHaveBeenCalled();

            const enabledCleanup = clickOutside(onOutside, { enabled: true })(host);
            expect(docListeners.activeCount('pointerdown')).toBe(1);

            inner.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
            expect(onOutside).not.toHaveBeenCalled();

            document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
            expect(onOutside).toHaveBeenCalledOnce();

            enabledCleanup?.();
            expect(docListeners.activeCount('pointerdown')).toBe(0);
        } finally {
            host.remove();
        }
    });

    it('attaches SelectInput pointerdown listener only while open and closes on outside click', async () => {
        render(SelectInput, {
            props: {
                value: 'a',
                onchange: vi.fn(),
                options: [
                    { label: 'Alpha', value: 'a' },
                    { label: 'Beta', value: 'b' },
                ],
            },
        });

        const closedCount = docListeners.activeCount('pointerdown');

        await fireEvent.click(screen.getByRole('button', { name: /alpha/i }));
        expect(screen.getByRole('button', { name: 'Beta' })).toBeTruthy();
        expect(docListeners.activeCount('pointerdown')).toBe(closedCount + 1);

        await fireEvent.pointerDown(screen.getByRole('button', { name: 'Beta' }));
        expect(screen.getByRole('button', { name: 'Beta' })).toBeTruthy();

        await fireEvent.pointerDown(document.body);
        expect(screen.queryByRole('button', { name: 'Beta' })).toBeNull();
        expect(docListeners.activeCount('pointerdown')).toBe(closedCount);
    });

    it('attaches MultiSelect pointerdown listener only while open and closes on outside click', async () => {
        render(MultiSelect, {
            props: {
                values: [],
                onchange: vi.fn(),
                options: [{ label: 'Work', value: 'work' }],
            },
        });

        const closedCount = docListeners.activeCount('pointerdown');

        await fireEvent.click(screen.getByRole('button', { name: /select\.\.\./i }));
        expect(screen.getByRole('checkbox', { name: 'Work' })).toBeTruthy();
        expect(docListeners.activeCount('pointerdown')).toBe(closedCount + 1);

        await fireEvent.pointerDown(screen.getByRole('checkbox', { name: 'Work' }));
        expect(screen.getByRole('checkbox', { name: 'Work' })).toBeTruthy();

        await fireEvent.pointerDown(document.body);
        expect(screen.queryByRole('checkbox', { name: 'Work' })).toBeNull();
        expect(docListeners.activeCount('pointerdown')).toBe(closedCount);
    });

    it('attaches MoreScreensDropdown pointerdown listener only while open and closes on outside click', async () => {
        render(MoreScreensDropdown);

        const closedCount = docListeners.activeCount('pointerdown');

        await fireEvent.click(screen.getByTitle('More screens'));
        expect(screen.getByRole('button', { name: 'stats' })).toBeTruthy();
        expect(docListeners.activeCount('pointerdown')).toBe(closedCount + 1);

        await fireEvent.pointerDown(document.body);
        expect(screen.queryByRole('button', { name: 'stats' })).toBeNull();
        expect(docListeners.activeCount('pointerdown')).toBe(closedCount);
    });

    it('attaches CalendarHeader view menu listener only while open and detaches when closing', async () => {
        vi.useFakeTimers();
        render(CalendarHeader, {
            props: {
                titleLabel: 'May 2026',
                today: new Date(2026, 4, 15),
                view: DateGridView.ThreeWeeks,
                setView: vi.fn(),
                setAnchor: vi.fn(),
                shift: vi.fn(),
            },
        });

        const closedCount = docListeners.activeCount('pointerdown');

        await fireEvent.click(screen.getByTitle('View options'));
        expect(screen.getByRole('button', { name: 'month' })).toBeTruthy();
        expect(docListeners.activeCount('pointerdown')).toBe(closedCount + 1);

        await fireEvent.pointerDown(document.body);
        expect(docListeners.activeCount('pointerdown')).toBe(closedCount);

        await vi.advanceTimersByTimeAsync(200);
        expect(screen.queryByRole('button', { name: 'month' })).toBeNull();
    });

    it('attaches TimelineControls view menu listener only while open and closes on outside click', async () => {
        render(TimelineControls, {
            props: {
                viewDate: new Date(2026, 4, 15),
                today: new Date(2026, 4, 15),
                setTimelineDate: vi.fn(),
                numDays: 3,
                setNumDays: vi.fn(),
            },
        });

        const closedCount = docListeners.activeCount('pointerdown');

        await fireEvent.click(screen.getByRole('button', { name: 'View columns' }));
        expect(screen.getByRole('button', { name: '3 days' })).toBeTruthy();
        expect(docListeners.activeCount('pointerdown')).toBe(closedCount + 1);

        await fireEvent.pointerDown(document.body);
        expect(screen.queryByRole('button', { name: '3 days' })).toBeNull();
        expect(docListeners.activeCount('pointerdown')).toBe(closedCount);
    });

    it('keeps InspectorPane listener-free while closed, respects popovers, and closes on outside click', async () => {
        const onClose = vi.fn();
        const draft = makeEvent({ id: 'ev-1', title: 'Review' });
        const { rerender } = render(InspectorPane, {
            props: {
                inspectorState: undefined,
                onClose,
            },
        });

        const closedPointerCount = docListeners.activeCount('pointerdown');
        const closedKeydownCount = docListeners.activeCount('keydown');

        await rerender({
            inspectorState: { type: 'event', id: 'ev-1', draft },
            onClose,
        });
        await screen.findByDisplayValue('Review');
        expect(docListeners.activeCount('pointerdown')).toBe(closedPointerCount + 1);
        expect(docListeners.activeCount('keydown')).toBe(closedKeydownCount + 1);

        const popover = document.createElement('div');
        popover.setAttribute('data-inspector-popover', '');
        document.body.appendChild(popover);
        try {
            await fireEvent.pointerDown(popover);
            expect(onClose).not.toHaveBeenCalled();
        } finally {
            popover.remove();
        }

        await fireEvent.pointerDown(screen.getByDisplayValue('Review'));
        expect(onClose).not.toHaveBeenCalled();

        await fireEvent.pointerDown(document.body);
        expect(onClose).toHaveBeenCalledOnce();

        await rerender({ inspectorState: undefined, onClose });
        expect(docListeners.activeCount('pointerdown')).toBe(closedPointerCount);
        expect(docListeners.activeCount('keydown')).toBe(closedKeydownCount);
    });

    it('dismisses TaskContextMenu on outside capture pointerdown and cleans up on unmount', async () => {
        const onClose = vi.fn();
        const { unmount } = render(TaskContextMenu, {
            props: {
                task: { id: 't1', title: 'Ship it' },
                x: 10,
                y: 10,
                onClose,
                onDelete: vi.fn(),
                onDo: vi.fn(),
            },
        });

        expect(winListeners.activeCount('pointerdown')).toBe(1);

        await fireEvent.pointerDown(screen.getByRole('menuitem', { name: 'Do' }));
        expect(onClose).not.toHaveBeenCalled();

        const blocker = document.createElement('div');
        blocker.addEventListener('pointerdown', (e) => { e.stopPropagation(); });
        document.body.appendChild(blocker);
        try {
            await fireEvent.pointerDown(blocker);
            expect(onClose).toHaveBeenCalledOnce();
        } finally {
            blocker.remove();
        }

        unmount();
        expect(winListeners.activeCount('pointerdown')).toBe(0);
    });
});
