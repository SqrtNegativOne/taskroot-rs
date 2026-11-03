import {
    currentMonitor,
    getCurrentWindow,
    LogicalPosition,
    LogicalSize,
} from '@tauri-apps/api/window';
import { computeMenuLayout, type WorkArea } from './menuLayout';

export interface StoredPosition {
    readonly x: number;
    readonly y: number;
}

export interface BarSize {
    readonly width: number;
    readonly height: number;
}

interface ControllerOptions {
    readonly savePosition: (position: StoredPosition) => void;
}

interface ScreenRect {
    readonly left: number;
    readonly right: number;
    readonly top: number;
    readonly bottom: number;
}

const SNAP_THRESHOLD = 24;

function workAreaOf(monitor: Awaited<ReturnType<typeof currentMonitor>>): WorkArea | null {
    if (monitor === null) return null;
    const factor = monitor.scaleFactor;
    const top = monitor.workArea.position.y / factor;
    return {
        top,
        bottom: top + monitor.workArea.size.height / factor,
    };
}

export function isTaskMenuTarget(target: EventTarget | null): boolean {
    return target instanceof Element && target.closest('[data-task-menu]') !== null;
}

export function isInteractiveTarget(target: EventTarget | null): boolean {
    return target instanceof Element && target.closest('button, [data-task-menu]') !== null;
}

export function createMinitrackerController(options: ControllerOptions) {
    let menuOpen = $state(false);
    let menuPlacement = $state<'up' | 'down'>('down');
    let isClickDimmed = $state(false);
    let menuAnchor: StoredPosition = { x: 0, y: 0 };
    let menuBarSize: BarSize = { width: 0, height: 0 };

    let isDragging = false;
    let readyToDrag = false;
    let dragStartX = 0;
    let dragStartY = 0;
    let initialWinX = 0;
    let initialWinY = 0;
    let monitorRect: ScreenRect | null = null;
    let winSize: { width: number; height: number } | null = null;
    let pendingX = 0;
    let pendingY = 0;
    let isSettingPosition = false;
    let wantsToSetPosition = false;

    async function openMenu(rowCount: number, barSize: BarSize): Promise<void> {
        const appWindow = getCurrentWindow();
        const [position, factor, monitor] = await Promise.all([
            appWindow.outerPosition(),
            appWindow.scaleFactor(),
            currentMonitor(),
        ]);

        menuAnchor = { x: position.x / factor, y: position.y / factor };
        menuBarSize = barSize;
        const layout = computeMenuLayout(menuAnchor.y, rowCount, workAreaOf(monitor), barSize.height);
        menuPlacement = layout.placement;
        menuOpen = true;

        await appWindow.setSize(new LogicalSize(barSize.width, layout.windowHeight));
        await appWindow.setPosition(new LogicalPosition(menuAnchor.x, layout.windowY));
    }

    async function closeMenu(): Promise<void> {
        if (!menuOpen) return;
        menuOpen = false;

        const appWindow = getCurrentWindow();
        await appWindow.setSize(new LogicalSize(menuBarSize.width, menuBarSize.height));
        await appWindow.setPosition(new LogicalPosition(menuAnchor.x, menuAnchor.y));
    }

    function toggleMenu(rowCount: number, barSize: BarSize): void {
        void (menuOpen ? closeMenu() : openMenu(rowCount, barSize));
    }

    async function pointerDown(event: PointerEvent): Promise<void> {
        if (event.button !== 0 || isInteractiveTarget(event.target)) return;

        const target = event.currentTarget as HTMLElement;
        target.setPointerCapture(event.pointerId);

        isDragging = true;
        readyToDrag = false;
        isClickDimmed = true;
        dragStartX = event.screenX;
        dragStartY = event.screenY;

        if (menuOpen) {
            await closeMenu();
        }

        const appWindow = getCurrentWindow();
        const [pos, factor, size, monitor] = await Promise.all([
            appWindow.outerPosition(),
            appWindow.scaleFactor(),
            appWindow.outerSize(),
            currentMonitor(),
        ]);

        if (!isDragging) return;

        initialWinX = pos.x / factor;
        initialWinY = pos.y / factor;
        winSize = { width: size.width / factor, height: size.height / factor };

        if (monitor) {
            const mFactor = monitor.scaleFactor;
            monitorRect = {
                left: monitor.workArea.position.x / mFactor,
                top: monitor.workArea.position.y / mFactor,
                right: (monitor.workArea.position.x + monitor.workArea.size.width) / mFactor,
                bottom: (monitor.workArea.position.y + monitor.workArea.size.height) / mFactor,
            };
        } else {
            monitorRect = null;
        }

        readyToDrag = true;
        pendingX = initialWinX;
        pendingY = initialWinY;
    }

    async function updatePosition(): Promise<void> {
        if (isSettingPosition || !wantsToSetPosition) return;
        isSettingPosition = true;
        wantsToSetPosition = false;
        try {
            await getCurrentWindow().setPosition(new LogicalPosition(pendingX, pendingY));
        } finally {
            isSettingPosition = false;
            if (wantsToSetPosition) {
                requestAnimationFrame(() => { void updatePosition(); });
            }
        }
    }

    function pointerMove(event: PointerEvent): void {
        if (!isDragging || !readyToDrag) return;

        let newX = initialWinX + (event.screenX - dragStartX);
        let newY = initialWinY + (event.screenY - dragStartY);

        if (monitorRect && winSize) {
            if (Math.abs(newX - monitorRect.left) < SNAP_THRESHOLD) {
                newX = monitorRect.left;
            } else if (Math.abs((newX + winSize.width) - monitorRect.right) < SNAP_THRESHOLD) {
                newX = monitorRect.right - winSize.width;
            }

            if (Math.abs(newY - monitorRect.top) < SNAP_THRESHOLD) {
                newY = monitorRect.top;
            } else if (Math.abs((newY + winSize.height) - monitorRect.bottom) < SNAP_THRESHOLD) {
                newY = monitorRect.bottom - winSize.height;
            }
        }

        pendingX = newX;
        pendingY = newY;
        wantsToSetPosition = true;
        void updatePosition();
    }

    function pointerUp(event: PointerEvent): void {
        if (!isDragging) return;
        const wasDraggable = readyToDrag;
        isDragging = false;
        readyToDrag = false;
        isClickDimmed = false;
        const target = event.currentTarget as HTMLElement;
        target.releasePointerCapture(event.pointerId);
        if (wasDraggable) {
            options.savePosition({ x: pendingX, y: pendingY });
        }
    }

    return {
        get menuOpen() {
            return menuOpen;
        },
        get menuPlacement() {
            return menuPlacement;
        },
        get isClickDimmed() {
            return isClickDimmed;
        },
        toggleMenu,
        closeMenu,
        pointerDown,
        pointerMove,
        pointerUp,
    };
}
