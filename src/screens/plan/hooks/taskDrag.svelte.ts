import type { AppTask } from '../../../lib/domain';
import type { DragState, DragStateTarget } from '../day-timeline/types';
import { clampDuration } from '../snapping';
import { createDropDragController } from './dropDrag.svelte';

/** Where a task dropped on a whole-day cell starts. */
const GRID_DAY_START_MINUTE = 9 * 60;
const DEFAULT_BLOCK_MINUTES = 30;

export interface TaskScheduleSlot {
    date: string;
    minute: number;
    duration: number;
}

export interface TaskDragControllerOptions {
    setDragState: (state: DragState | undefined) => void;
    onSchedule: (task: AppTask, slot: TaskScheduleSlot) => void;
    /** Fallback block length when the task has no estimate, in minutes. */
    defaultDuration?: () => number;
}

export type TaskDragStart = (event: PointerEvent | MouseEvent, task: AppTask) => void;

function blockMinutes(task: AppTask, fallback: number): number {
    const minutes = task.est !== undefined && task.est > 0 ? task.est : fallback;
    return clampDuration(minutes);
}

function slotFor(target: DragStateTarget, duration: number): TaskScheduleSlot | undefined {
    if (!target.date) return undefined;
    const minute =
        target.kind === 'day-time' && target.minute !== undefined
            ? target.minute
            : GRID_DAY_START_MINUTE;
    return { date: target.date, minute, duration };
}

/**
 * Pointer-driven drag of a task row onto the grid or timeline. Dropping creates
 * a linked event; the returned starter is safe to pass as an event handler prop.
 */
export function createTaskDragController(
    options: TaskDragControllerOptions,
): TaskDragStart {
    return createDropDragController<AppTask, TaskScheduleSlot>({
        setDragState: options.setDragState,
        dragState: (task) => ({ task: { id: task.id } }),
        duration: (task) =>
            blockMinutes(task, options.defaultDuration?.() ?? DEFAULT_BLOCK_MINUTES),
        payloadFor: (task, target) => slotFor(target, blockMinutes(task, options.defaultDuration?.() ?? DEFAULT_BLOCK_MINUTES)),
        onDrop: options.onSchedule,
    });
}
