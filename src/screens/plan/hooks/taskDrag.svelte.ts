import type { AppTask } from '../../../lib/domain';
import type { DragState, DragStateTarget } from '../day-timeline/types';
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

export interface TaskDragControllerOptions {
    setDragState: (state: DragState | undefined) => void;
    onSetStart: (task: AppTask, date: string) => void;
    onSchedule: (task: AppTask, slot: TaskScheduleSlot) => void;
    /** Fallback block length when the task has no estimate, in minutes. */
    defaultDuration?: () => number;
}

export type TaskDragStart = (event: PointerEvent | MouseEvent, task: AppTask) => void;

function blockMinutes(task: AppTask, fallback: number): number {
    const minutes = task.est !== undefined && task.est > 0 ? task.est : fallback;
    return clampDuration(minutes);
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
    if (target.kind !== 'day-time' || target.minute === undefined) return undefined;
    return {
        kind: 'schedule',
        slot: { date: target.date, minute: target.minute, duration },
    };
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
