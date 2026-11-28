import { computeMenuLayout } from './menuLayout';
import {
    createTauriWindowGeometryAdapter,
    monitorRectOf,
    snapToMonitor,
    workAreaOf,
    type BarSize,
    type ScreenRect,
    type StoredPosition,
    type WindowGeometryAdapter,
} from './windowGeometry';

export type {
    BarSize,
    MonitorWorkArea,
    PhysicalRect,
    StoredPosition,
    WindowGeometryAdapter,
} from './windowGeometry';
export { createTauriWindowGeometryAdapter } from './windowGeometry';

export interface ControllerOptions {
    readonly savePosition: (position: StoredPosition) => void;
    readonly geometry?: WindowGeometryAdapter;
}

export function isTaskMenuTarget(target: EventTarget | null): boolean {
    return target instanceof Element && target.closest('[data-task-menu]') !== null;
}

export function isInteractiveTarget(target: EventTarget | null): boolean {
    return target instanceof Element && target.closest('button, [data-task-menu]') !== null;
}

class MinitrackerController {
    menuOpen = $state(false);
    menuPlacement = $state<'up' | 'down'>('down');
    isClickDimmed = $state(false);

    private readonly geometry: WindowGeometryAdapter;
    private readonly savePosition: (position: StoredPosition) => void;
    private menuAnchor: StoredPosition = { x: 0, y: 0 };
    private menuBarSize: BarSize = { width: 0, height: 0 };
    private menuIntentOpen = false;
    private transitionGen = 0;
    private transitionQueue: Promise<void> = Promise.resolve();

    private isDragging = false;
    private readyToDrag = false;
    private dragStartX = 0;
    private dragStartY = 0;
    private initialWinX = 0;
    private initialWinY = 0;
    private monitorRect: ScreenRect | null = null;
    private winSize: BarSize | null = null;
    private pendingX = 0;
    private pendingY = 0;
    private isSettingPosition = false;
    private wantsToSetPosition = false;

    constructor(options: ControllerOptions) {
        this.savePosition = options.savePosition;
        this.geometry = options.geometry ?? createTauriWindowGeometryAdapter();
    }

    private enqueue(task: () => Promise<void>): Promise<void> {
        const next = this.transitionQueue.then(task, task);
        this.transitionQueue = next.then(
            () => undefined,
            () => undefined,
        );
        return next;
    }

    private async runOpenMenu(
        gen: number,
        rowCount: number,
        barSize: BarSize,
        extraContentHeight: number,
    ): Promise<void> {
        if (gen !== this.transitionGen) return;
        const [position, factor, monitor] = await Promise.all([
            this.geometry.outerPosition(),
            this.geometry.scaleFactor(),
            this.geometry.currentMonitor(),
        ]);
        if (gen !== this.transitionGen) return;

        this.menuAnchor = { x: position.x / factor, y: position.y / factor };
        this.menuBarSize = barSize;
        const layout = computeMenuLayout(
            this.menuAnchor.y,
            rowCount,
            workAreaOf(monitor),
            barSize.height,
            extraContentHeight,
        );
        this.menuPlacement = layout.placement;
        this.menuOpen = true;

        await this.geometry.setSize(barSize.width, layout.windowHeight);
        await this.geometry.setPosition(this.menuAnchor.x, layout.windowY);
    }

    private runCloseMenu = async (): Promise<void> => {
        if (!this.menuOpen) return;
        this.menuOpen = false;
        await this.geometry.setSize(this.menuBarSize.width, this.menuBarSize.height);
        await this.geometry.setPosition(this.menuAnchor.x, this.menuAnchor.y);
    };

    private async runPointerDown(gen: number): Promise<void> {
        if (this.menuOpen) {
            await this.runCloseMenu();
        }
        if (!this.isDragging || gen !== this.transitionGen) return;

        const [pos, factor, size, monitor] = await Promise.all([
            this.geometry.outerPosition(),
            this.geometry.scaleFactor(),
            this.geometry.outerSize(),
            this.geometry.currentMonitor(),
        ]);
        if (!this.isDragging || gen !== this.transitionGen) return;

        this.initialWinX = pos.x / factor;
        this.initialWinY = pos.y / factor;
        this.winSize = { width: size.width / factor, height: size.height / factor };
        this.monitorRect = monitorRectOf(monitor);
        this.readyToDrag = true;
        this.pendingX = this.initialWinX;
        this.pendingY = this.initialWinY;
    }

    private async flushPosition(): Promise<void> {
        if (this.isSettingPosition || !this.wantsToSetPosition) return;
        this.isSettingPosition = true;
        this.wantsToSetPosition = false;
        try {
            await this.geometry.setPosition(this.pendingX, this.pendingY);
        } finally {
            this.isSettingPosition = false;
            if (this.wantsToSetPosition) {
                requestAnimationFrame(() => {
                    void this.flushPosition();
                });
            }
        }
    }

    closeMenu = (): Promise<void> => {
        this.menuIntentOpen = false;
        ++this.transitionGen;
        return this.enqueue(this.runCloseMenu);
    };

    toggleMenu = (rowCount: number, barSize: BarSize, extraContentHeight = 0): Promise<void> => {
        if (this.menuOpen || this.menuIntentOpen) {
            return this.closeMenu();
        }
        this.menuIntentOpen = true;
        const gen = ++this.transitionGen;
        return this.enqueue(() => this.runOpenMenu(gen, rowCount, barSize, extraContentHeight));
    };

    pointerDown = (event: PointerEvent): Promise<void> => {
        if (event.button !== 0 || isInteractiveTarget(event.target)) return Promise.resolve();

        const target = event.currentTarget as HTMLElement;
        target.setPointerCapture(event.pointerId);

        this.isDragging = true;
        this.readyToDrag = false;
        this.isClickDimmed = true;
        this.dragStartX = event.screenX;
        this.dragStartY = event.screenY;
        this.menuIntentOpen = false;
        const gen = ++this.transitionGen;

        return this.enqueue(() => this.runPointerDown(gen));
    };

    pointerMove = (event: PointerEvent): void => {
        if (!this.isDragging || !this.readyToDrag) return;
        const rawX = this.initialWinX + (event.screenX - this.dragStartX);
        const rawY = this.initialWinY + (event.screenY - this.dragStartY);
        const snapped = snapToMonitor(rawX, rawY, this.monitorRect, this.winSize);
        this.pendingX = snapped.x;
        this.pendingY = snapped.y;
        this.wantsToSetPosition = true;
        void this.flushPosition();
    };

    pointerUp = (event: PointerEvent): void => {
        if (!this.isDragging) return;
        const wasDraggable = this.readyToDrag;
        this.isDragging = false;
        this.readyToDrag = false;
        this.isClickDimmed = false;
        this.wantsToSetPosition = false;
        ++this.transitionGen;
        const target = event.currentTarget as HTMLElement;
        target.releasePointerCapture(event.pointerId);
        if (wasDraggable) {
            this.savePosition({ x: this.pendingX, y: this.pendingY });
        }
    };
}

export function createMinitrackerController(options: ControllerOptions) {
    return new MinitrackerController(options);
}
