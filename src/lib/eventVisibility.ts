import type { EventInstance } from './domain';

export interface EventVisibility {
    hideFrequentRecurring: boolean;
    hideTimed: boolean;
}

const FREQUENT_FREQUENCIES = new Set(['SECONDLY', 'MINUTELY', 'HOURLY', 'DAILY', 'WEEKLY']);

function recurrenceFrequency(rrule?: string | null): string | undefined {
    return /\bFREQ=([A-Z]+)/i.exec(rrule ?? '')?.[1]?.toUpperCase();
}

/** True when the instance repeats at least as often as once a week. */
export function isFrequentRecurrence(event: EventInstance): boolean {
    const frequency = recurrenceFrequency(event.rrule);
    return frequency !== undefined && FREQUENT_FREQUENCIES.has(frequency);
}

export function isEventVisibility(value: unknown): value is EventVisibility {
    return typeof value === 'object' && value !== null
        && 'hideFrequentRecurring' in value && typeof value.hideFrequentRecurring === 'boolean'
        && 'hideTimed' in value && typeof value.hideTimed === 'boolean';
}

export function isEventVisible(event: EventInstance, visibility: EventVisibility): boolean {
    if (visibility.hideFrequentRecurring && isFrequentRecurrence(event)) return false;
    return !(visibility.hideTimed && event.timing.kind === 'timed');
}
