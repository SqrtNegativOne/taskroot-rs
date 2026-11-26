import type { EventInstance } from '../../../lib/domain';
import { DRAG_THRESHOLD_PX } from '../day-timeline/constants';
import { createPointerGestureRecognizer } from '../day-timeline/hooks/pointerGesture.svelte';
import { suppressUpcomingClick } from '../date-grid/dropTarget';
import {
    resizeAllDayRange,
    type AllDayRange,
    type SpanEdge,
} from '../date-grid/spanResize';

export interface SpanResizePreview extends AllDayRange {
    occurrenceKey: string;
    edge: SpanEdge;
}

export interface SpanResizeOptions {
    onPreview: (preview: SpanResizePreview | undefined) => void;
    onCommit: (instance: EventInstance, range: AllDayRange) => void;
}

export type SpanResizeStart = (
    downEvent: PointerEvent,
    instance: EventInstance,
    edge: SpanEdge,
    dateAt: (clientX: number) => string | undefined,
) => void;

/** Pointer gesture that stretches a multi-day all-day bar from one end. */
export function createSpanResizeController(options: SpanResizeOptions): SpanResizeStart {
    const track = createPointerGestureRecognizer();

    return (downEvent, instance, edge, dateAt) => {
        if (downEvent.button !== 0) return;
        downEvent.preventDefault();
        downEvent.stopPropagation();

        const startX = downEvent.clientX;
        const startY = downEvent.clientY;
        let dragging = false;
        let range: AllDayRange | undefined;

        const update = (clientX: number) => {
            const candidate = dateAt(clientX);
            if (!candidate) return;
            range = resizeAllDayRange(instance, edge, candidate);
            if (range) options.onPreview({ occurrenceKey: instance.occurrenceKey, edge, ...range });
        };

        track({
            onMove: (move) => {
                if (!dragging) {
                    const moved = Math.hypot(move.clientX - startX, move.clientY - startY);
                    if (moved < DRAG_THRESHOLD_PX) return;
                    dragging = true;
                }
                update(move.clientX);
            },
            onEnd: () => {
                options.onPreview(undefined);
                if (!dragging) return;
                suppressUpcomingClick();
                if (range) options.onCommit(instance, range);
            },
            onCancel: () => {
                options.onPreview(undefined);
            },
        });
    };
}
