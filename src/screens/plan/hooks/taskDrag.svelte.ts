import type { AppTask } from '../../../lib/domain';
import { DRAG_THRESHOLD_PX, SNAP_MIN } from '../day-timeline/constants';
import { createPointerGestureRecognizer } from '../day-timeline/hooks/pointerGesture.svelte';
import type { DragState, DragStateTarget } from '../day-timeline/types';
import { MINUTES_PER_DAY } from '../date-grid/dropPayload';
import { dropTargetAt, suppressUpcomingClick } from '../date-grid/dropTarget';

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
    return Math.min(MINUTES_PER_DAY, Math.max(SNAP_MIN, minutes));
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
    const track = createPointerGestureRecognizer();

    return (downEvent, task) => {
        if (downEvent.button !== 0) return;
        downEvent.preventDefault();

        const startX = downEvent.clientX;
        const startY = downEvent.clientY;
        const duration = blockMinutes(
            task,
            options.defaultDuration?.() ?? DEFAULT_BLOCK_MINUTES,
        );
        let dragging = false;
        let target: DragStateTarget | undefined;

        track({
            onMove: (move) => {
                if (!dragging) {
                    const moved = Math.hypot(move.clientX - startX, move.clientY - startY);
                    if (moved < DRAG_THRESHOLD_PX) return;
                    dragging = true;
                }
                target = dropTargetAt(move.clientX, move.clientY, duration);
                options.setDragState({ task: { id: task.id }, target });
            },
            onEnd: () => {
                options.setDragState(undefined);
                if (!dragging) return;
                suppressUpcomingClick();
                if (!target) return;
                const slot = slotFor(target, duration);
                if (slot) options.onSchedule(task, slot);
            },
            onCancel: () => {
                options.setDragState(undefined);
            },
        });
    };
}
