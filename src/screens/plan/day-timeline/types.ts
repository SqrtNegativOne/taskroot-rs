export const MINUTES_IN_HOUR = 60;
export const HOURS_PER_DAY = 24;

export const PIXELS_PER_HOUR = 56;
export const PX_PER_MIN = PIXELS_PER_HOUR / MINUTES_IN_HOUR; // 56 / 60
export const SNAP_MIN = 15;

export interface DragStateTarget {
    kind: string;
    minute?: number;
    duration?: number;
    date?: string;
    start?: number;
    end?: number;
    dragOffsetMins?: number;
}

export interface DragState {
    target?: DragStateTarget;
    event?: unknown;
}

import type { AppEvent } from '../../../lib/domain';

export interface LaidEvent {
    event: AppEvent;
    startMins: number;
    endMins: number;
    lane: number;
    lanes: number;
}

export interface PlanDayLayout {
    date: string;
    events: LaidEvent[];
}
