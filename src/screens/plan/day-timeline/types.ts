import type { EventInstance } from '../../../lib/domain';

export interface LaidEvent {
    event: EventInstance;
    startMins: number;
    endMins: number;
    lane: number;
    lanes: number;
}

/** The minute range of a creation ghost before it becomes a stored event. */
export interface CreationRange {
    start: number;
    end: number;
}

/** A bucketed event before `layoutEvents` assigns it a lane. */
export type UnlaidEvent = Omit<LaidEvent, 'lane' | 'lanes'>;

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
    task?: { id: string };
}
