import type { AppEvent } from '../../../lib/domain';

export interface LaidEvent {
    event: AppEvent;
    startMins: number;
    endMins: number;
    lane: number;
    lanes: number;
}

export {
    MINUTES_IN_HOUR,
    HOURS_PER_DAY,
    PIXELS_PER_HOUR,
    PX_PER_MIN,
    SNAP_MIN,
    COMPACT_EVENT_HEIGHT_PX,
    DRAG_THRESHOLD_PX,
} from './constants';

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
    event?: { id: string };
}
