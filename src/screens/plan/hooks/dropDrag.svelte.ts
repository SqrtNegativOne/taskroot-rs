import { DRAG_THRESHOLD_PX } from '../day-timeline/constants';
import { createPointerGestureRecognizer } from '../day-timeline/hooks/pointerGesture.svelte';
import type { DragState, DragStateTarget } from '../day-timeline/types';
import { dropTargetAt, suppressUpcomingClick } from '../date-grid/dropTarget';

export interface DropDragOptions<TItem, TPayload> {
    setDragState: (state: DragState | undefined) => void;
    dragState: (item: TItem) => DragState;
    duration: (item: TItem) => number;
    payloadFor: (item: TItem, target: DragStateTarget) => TPayload | undefined;
    onDrop: (item: TItem, payload: TPayload) => void;
}

/**
 * The shared pointer-drag pipeline: threshold, target tracking, shared drag
 * state, click suppression and the drop. Each controller keeps only its payload
 * conversion and drop effect.
 */
export function createDropDragController<TItem, TPayload>(
    options: DropDragOptions<TItem, TPayload>,
): (downEvent: PointerEvent | MouseEvent, item: TItem) => void {
    const track = createPointerGestureRecognizer();

    return (downEvent, item) => {
        if (downEvent.button !== 0) return;
        downEvent.preventDefault();

        const startX = downEvent.clientX;
        const startY = downEvent.clientY;
        const duration = options.duration(item);
        const state = options.dragState(item);
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
                options.setDragState({ ...state, target });
            },
            onEnd: () => {
                options.setDragState(undefined);
                if (!dragging) return;
                suppressUpcomingClick();
                if (!target) return;
                const payload = options.payloadFor(item, target);
                if (payload) options.onDrop(item, payload);
            },
            onCancel: () => {
                options.setDragState(undefined);
            },
        });
    };
}
