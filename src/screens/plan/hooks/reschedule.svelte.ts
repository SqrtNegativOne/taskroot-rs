import type { EventInstance, EventScope } from '../../../lib/domain';
import { store } from '../../../lib/store.svelte';

type PendingReschedule = {
    instance: EventInstance;
    startTime: string;
    endTime: string;
    isAllDay: boolean;
    operation: string;
};

/**
 * The recurring-move prompt shared by the plan screen and the timeline: a
 * one-off event is committed immediately, a recurring one asks for a scope
 * first. The modal reads `isOpen` and calls `confirm`/`cancel`.
 */
export function createReschedulePrompt() {
    let pending = $state<PendingReschedule | undefined>(undefined);
    let isOpen = $state(false);

    function commit(
        instance: EventInstance,
        startTime: string,
        endTime: string,
        isAllDay: boolean,
        scope: EventScope,
        operation: string,
    ) {
        void store.mutate(operation, () =>
            store.rescheduleEvent(
                instance.id,
                startTime,
                endTime,
                isAllDay,
                scope,
                instance.occurrenceStart,
            ),
        );
    }

    function request(
        instance: EventInstance,
        startTime: string,
        endTime: string,
        isAllDay: boolean,
        operation: string,
    ) {
        if (!instance.recurring) {
            commit(instance, startTime, endTime, isAllDay, 'all', operation);
            return;
        }
        pending = { instance, startTime, endTime, isAllDay, operation };
        isOpen = true;
    }

    function confirm(scope: EventScope) {
        isOpen = false;
        const move = pending;
        pending = undefined;
        if (!move) return;
        commit(
            move.instance,
            move.startTime,
            move.endTime,
            move.isAllDay,
            scope,
            move.operation,
        );
    }

    function cancel() {
        isOpen = false;
        pending = undefined;
    }

    return {
        get isOpen() {
            return isOpen;
        },
        request,
        confirm,
        cancel,
    };
}
