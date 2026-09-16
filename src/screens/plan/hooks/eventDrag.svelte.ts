import type { EventInstance } from '../../../lib/domain';
import { DRAG_THRESHOLD_PX, SNAP_MIN } from '../day-timeline/constants';
import { createPointerGestureRecognizer } from '../day-timeline/hooks/pointerGesture.svelte';
import type { DragState, DragStateTarget } from '../day-timeline/types';
import {
    DEFAULT_DROP_DURATION_MINUTES,
    gridDayDropPayload,
    snappedDropMinute,
    timelineDropPayload,
    type ReschedulePayload,
} from '../date-grid/dropPayload';

const MS_PER_MINUTE = 60_000;

export interface EventDragControllerOptions {
    setDragState: (state: DragState | undefined) => void;
    onReschedule: (event: EventInstance, payload: ReschedulePayload) => void;
}

export type EventDragStart = (event: PointerEvent, instance: EventInstance) => void;

function durationMinutes(instance: EventInstance): number {
    if (instance.timing.kind === 'allDay') return DEFAULT_DROP_DURATION_MINUTES;
    const span =
        new Date(instance.timing.end).getTime() - new Date(instance.timing.start).getTime();
    return Math.max(SNAP_MIN, Math.round(span / MS_PER_MINUTE));
}

function targetFromElement(
    element: Element,
    clientY: number,
    duration: number,
): DragStateTarget | undefined {
    const kind = element.getAttribute('data-drop-kind');
    const date = element.getAttribute('data-drop-date');
    if (!date) return undefined;
    if (kind === 'grid-day') return { kind, date };
    if (kind !== 'day-time') return undefined;

    const rect = element.getBoundingClientRect();
    const minute = snappedDropMinute(clientY - rect.top, duration);
    return { kind, date, minute, duration };
}

function dropTargetAt(
    clientX: number,
    clientY: number,
    duration: number,
): DragStateTarget | undefined {
    const element = document.elementFromPoint(clientX, clientY)?.closest('[data-drop-kind]');
    return element ? targetFromElement(element, clientY, duration) : undefined;
}

function payloadFor(
    instance: EventInstance,
    target: DragStateTarget,
): ReschedulePayload | undefined {
    if (!target.date) return undefined;
    if (target.kind === 'day-time' && target.minute !== undefined) {
        return timelineDropPayload(instance, target.date, target.minute);
    }
    if (target.kind === 'grid-day') {
        return gridDayDropPayload(instance, target.date);
    }
    return undefined;
}

/** Swallow the click that follows a completed drag, so it does not open the inspector. */
function suppressUpcomingClick() {
    const suppress = (click: Event) => {
        click.stopPropagation();
        click.preventDefault();
    };
    window.addEventListener('click', suppress, true);
    setTimeout(() => {
        window.removeEventListener('click', suppress, true);
    }, 0);
}

/**
 * Pointer-driven drag of a date-grid event. The recognizer must be created
 * during component init, so the returned starter is safe to pass as an event
 * handler prop.
 */
export function createEventDragController(
    options: EventDragControllerOptions,
): EventDragStart {
    const track = createPointerGestureRecognizer();

    return (downEvent, instance) => {
        if (downEvent.button !== 0) return;
        downEvent.preventDefault();

        const startX = downEvent.clientX;
        const startY = downEvent.clientY;
        const duration = durationMinutes(instance);
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
                options.setDragState({ event: { id: instance.id }, target });
            },
            onEnd: () => {
                options.setDragState(undefined);
                if (!dragging) return;
                suppressUpcomingClick();
                if (!target) return;
                const payload = payloadFor(instance, target);
                if (payload) options.onReschedule(instance, payload);
            },
            onCancel: () => {
                options.setDragState(undefined);
            },
        });
    };
}
