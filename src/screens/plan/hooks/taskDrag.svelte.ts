import type { AppTask, TaskDateInstance, TaskDateKind } from '../../../lib/domain';
import type { DragState, DragStateTarget } from '../day-timeline/types';
import { movedTaskDateValue } from '../date-grid/dropPayload';
import { clampDuration } from '../snapping';
import { createDropDragController } from './dropDrag.svelte';

const DEFAULT_BLOCK_MINUTES = 30;

export interface TaskScheduleSlot {
    date: string;
    minute: number;
    duration: number;
}

export type TaskDrop =
    | { kind: 'start'; date: string }
    | { kind: 'schedule'; slot: TaskScheduleSlot };

export type TaskMarkerDrop =
    | { kind: 'move'; field: TaskDateKind; value: string }
    | { kind: 'schedule'; slot: TaskScheduleSlot };

export interface TaskDragControllerOptions {
    setDragState: (state: DragState | undefined) => void;
    onSetStart: (task: AppTask, date: string) => void;
    onSchedule: (task: AppTask, slot: TaskScheduleSlot) => void;
    /** Fallback block length when the task has no estimate, in minutes. */
    defaultDuration?: () => number;
}

export interface TaskMarkerDragControllerOptions {
    setDragState: (state: DragState | undefined) => void;
    onMoveDate: (marker: TaskDateInstance, field: TaskDateKind, value: string) => void;
    onSchedule: (marker: TaskDateInstance, slot: TaskScheduleSlot) => void;
    /** Block length when dropping onto the timeline, in minutes. */
    defaultDuration?: () => number;
}

export type TaskDragStart = (event: PointerEvent | MouseEvent, task: AppTask) => void;
export type TaskMarkerDragStart = (
    event: PointerEvent | MouseEvent,
    marker: TaskDateInstance,
) => void;

function blockMinutes(task: AppTask, fallback: number): number {
    const minutes = task.est != null && task.est > 0 ? task.est : fallback;
    return clampDuration(minutes);
}

function scheduleSlotFor(
    target: DragStateTarget,
    duration: number,
): TaskScheduleSlot | undefined {
    if (!target.date || target.kind !== 'day-time' || target.minute === undefined) {
        return undefined;
    }
    return { date: target.date, minute: target.minute, duration };
}

/**
 * A grid day sets the task's start date; a timeline column schedules a linked
 * event at the dropped minute.
 */
export function taskDropFor(
    target: DragStateTarget,
    duration: number,
): TaskDrop | undefined {
    if (!target.date) return undefined;
    if (target.kind === 'grid-day') return { kind: 'start', date: target.date };
    const slot = scheduleSlotFor(target, duration);
    return slot ? { kind: 'schedule', slot } : undefined;
}

/**
 * A grid day moves a task marker's `start` or `due` date (preserving any time
 * of day); a timeline column schedules a linked event at the dropped minute.
 */
export function taskMarkerDropFor(
    marker: TaskDateInstance,
    target: DragStateTarget,
    duration: number,
): TaskMarkerDrop | undefined {
    if (!target.date) return undefined;
    if (target.kind === 'grid-day') {
        const value = movedTaskDateValue(marker.timing, target.date);
        return value ? { kind: 'move', field: marker.kind, value } : undefined;
    }
    const slot = scheduleSlotFor(target, duration);
    return slot ? { kind: 'schedule', slot } : undefined;
}

/**
 * Pointer-driven drag of a task row onto the grid or timeline. The returned
 * starter is safe to pass as an event handler prop.
 */
export function createTaskDragController(
    options: TaskDragControllerOptions,
): TaskDragStart {
    const durationFor = (task: AppTask) =>
        blockMinutes(task, options.defaultDuration?.() ?? DEFAULT_BLOCK_MINUTES);

    return createDropDragController<AppTask, TaskDrop>({
        setDragState: options.setDragState,
        dragState: (task) => ({ task: { id: task.id } }),
        duration: durationFor,
        payloadFor: (task, target) => taskDropFor(target, durationFor(task)),
        onDrop: (task, drop) => {
            if (drop.kind === 'start') {
                options.onSetStart(task, drop.date);
                return;
            }
            options.onSchedule(task, drop.slot);
        },
    });
}

/**
 * Pointer-driven drag of a date-grid task marker onto another day or timeline
 * column. The returned starter is safe to pass as an event handler prop.
 */
export function createTaskMarkerDragController(
    options: TaskMarkerDragControllerOptions,
): TaskMarkerDragStart {
    const durationFor = () =>
        clampDuration(options.defaultDuration?.() ?? DEFAULT_BLOCK_MINUTES);

    return createDropDragController<TaskDateInstance, TaskMarkerDrop>({
        setDragState: options.setDragState,
        dragState: (marker) => ({
            task: {
                id: marker.taskId,
                markerKind: marker.kind,
                occurrenceKey: marker.occurrenceKey,
            },
        }),
        duration: durationFor,
        payloadFor: (marker, target) => taskMarkerDropFor(marker, target, durationFor()),
        onDrop: (marker, drop) => {
            if (drop.kind === 'move') {
                options.onMoveDate(marker, drop.field, drop.value);
                return;
            }
            options.onSchedule(marker, drop.slot);
        },
    });
}
