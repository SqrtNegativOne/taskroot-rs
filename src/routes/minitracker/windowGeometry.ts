import {
    currentMonitor,
    getCurrentWindow,
    LogicalPosition,
    LogicalSize,
} from '@tauri-apps/api/window';
import type { WorkArea } from './menuLayout';

export interface StoredPosition {
    readonly x: number;
    readonly y: number;
}

export function isStoredPosition(value: unknown): value is StoredPosition {
    if (typeof value !== 'object' || value === null) return false;
    const candidate = value as { x?: unknown; y?: unknown };
    return typeof candidate.x === 'number' && typeof candidate.y === 'number';
}

export interface BarSize {
    readonly width: number;
    readonly height: number;
}

export interface PhysicalRect {
    readonly x: number;
    readonly y: number;
    readonly width: number;
    readonly height: number;
}

export interface MonitorWorkArea {
    readonly scaleFactor: number;
    readonly workArea: PhysicalRect;
}

export interface ScreenRect {
    readonly left: number;
    readonly right: number;
    readonly top: number;
    readonly bottom: number;
}

export interface WindowGeometryAdapter {
    outerPosition(): Promise<{ readonly x: number; readonly y: number }>;
    outerSize(): Promise<{ readonly width: number; readonly height: number }>;
    scaleFactor(): Promise<number>;
    currentMonitor(): Promise<MonitorWorkArea | null>;
    setSize(width: number, height: number): Promise<void>;
    setPosition(x: number, y: number): Promise<void>;
}

const SNAP_THRESHOLD = 24;

export function createTauriWindowGeometryAdapter(): WindowGeometryAdapter {
    return {
        outerPosition: () => getCurrentWindow().outerPosition(),
        outerSize: () => getCurrentWindow().outerSize(),
        scaleFactor: () => getCurrentWindow().scaleFactor(),
        async currentMonitor(): Promise<MonitorWorkArea | null> {
            const monitor = await currentMonitor();
            if (monitor === null) return null;
            return {
                scaleFactor: monitor.scaleFactor,
                workArea: {
                    x: monitor.workArea.position.x,
                    y: monitor.workArea.position.y,
                    width: monitor.workArea.size.width,
                    height: monitor.workArea.size.height,
                },
            };
        },
        setSize: (width, height) => getCurrentWindow().setSize(new LogicalSize(width, height)),
        setPosition: (x, y) => getCurrentWindow().setPosition(new LogicalPosition(x, y)),
    };
}

export function workAreaOf(monitor: MonitorWorkArea | null): WorkArea | null {
    if (monitor === null) return null;
    const factor = monitor.scaleFactor;
    const top = monitor.workArea.y / factor;
    return {
        top,
        bottom: top + monitor.workArea.height / factor,
    };
}

export function monitorRectOf(monitor: MonitorWorkArea | null): ScreenRect | null {
    if (monitor === null) return null;
    const factor = monitor.scaleFactor;
    return {
        left: monitor.workArea.x / factor,
        top: monitor.workArea.y / factor,
        right: (monitor.workArea.x + monitor.workArea.width) / factor,
        bottom: (monitor.workArea.y + monitor.workArea.height) / factor,
    };
}

export function snapToMonitor(
    x: number,
    y: number,
    rect: ScreenRect | null,
    size: BarSize | null,
): StoredPosition {
    if (rect === null || size === null) return { x, y };
    let nextX = x;
    let nextY = y;
    if (Math.abs(nextX - rect.left) < SNAP_THRESHOLD) {
        nextX = rect.left;
    } else if (Math.abs(nextX + size.width - rect.right) < SNAP_THRESHOLD) {
        nextX = rect.right - size.width;
    }
    if (Math.abs(nextY - rect.top) < SNAP_THRESHOLD) {
        nextY = rect.top;
    } else if (Math.abs(nextY + size.height - rect.bottom) < SNAP_THRESHOLD) {
        nextY = rect.bottom - size.height;
    }
    return { x: nextX, y: nextY };
}
