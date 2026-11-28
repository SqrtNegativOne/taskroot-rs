import {
    currentMonitor,
    getCurrentWindow,
    PhysicalPosition,
    PhysicalSize,
    primaryMonitor,
} from '@tauri-apps/api/window';

export const SIDEBAR_TAB_WIDTH = 18;
export const SIDEBAR_TAB_HEIGHT = 52;
export const SIDEBAR_PANEL_WIDTH = 328;
export const SIDEBAR_CLOSE_TRANSITION_MS = 250;

export interface SidebarMonitor {
    readonly scaleFactor: number;
    readonly position: { readonly x: number; readonly y: number };
    readonly size: { readonly width: number; readonly height: number };
}

export interface SidebarWindowBounds {
    readonly x: number;
    readonly y: number;
    readonly width: number;
    readonly height: number;
}

export interface SidebarWindowAdapter {
    show(): Promise<void>;
    unminimize(): Promise<void>;
    setFocus(): Promise<void>;
    currentMonitor(): Promise<SidebarMonitor | null>;
    primaryMonitor(): Promise<SidebarMonitor | null>;
    setPosition(x: number, y: number): Promise<void>;
    setSize(width: number, height: number): Promise<void>;
}

function mapMonitor(
    monitor: Awaited<ReturnType<typeof currentMonitor>>,
): SidebarMonitor | null {
    if (monitor === null) return null;
    return {
        scaleFactor: monitor.scaleFactor,
        position: { x: monitor.position.x, y: monitor.position.y },
        size: { width: monitor.size.width, height: monitor.size.height },
    };
}

export function createTauriSidebarWindowAdapter(): SidebarWindowAdapter {
    return {
        show: () => getCurrentWindow().show(),
        unminimize: () => getCurrentWindow().unminimize(),
        setFocus: () => getCurrentWindow().setFocus(),
        currentMonitor: async () => mapMonitor(await currentMonitor()),
        primaryMonitor: async () => mapMonitor(await primaryMonitor()),
        setPosition: (x, y) => getCurrentWindow().setPosition(new PhysicalPosition(x, y)),
        setSize: (width, height) => getCurrentWindow().setSize(new PhysicalSize(width, height)),
    };
}

export function computeSidebarBounds(
    monitor: SidebarMonitor,
    isOpen: boolean,
    tabTop: number,
): SidebarWindowBounds {
    const sf = monitor.scaleFactor;
    const screenX = monitor.position.x;
    const screenY = monitor.position.y;
    const screenW = monitor.size.width;
    const screenH = monitor.size.height;

    if (isOpen) {
        const width = Math.round((SIDEBAR_TAB_WIDTH + SIDEBAR_PANEL_WIDTH) * sf);
        return {
            x: screenX + screenW - width,
            y: screenY,
            width,
            height: screenH,
        };
    }

    const width = Math.round(SIDEBAR_TAB_WIDTH * sf);
    const height = Math.round(SIDEBAR_TAB_HEIGHT * sf);
    return {
        x: screenX + screenW - width,
        y: Math.round(screenY + tabTop * sf),
        width,
        height,
    };
}
