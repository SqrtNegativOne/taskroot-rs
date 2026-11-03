import type { EventInstance } from './domain';

export interface EventVisibility {
    hideRecurring: boolean;
    hideTimed: boolean;
}

export function isEventVisibility(value: unknown): value is EventVisibility {
    return typeof value === 'object' && value !== null
        && 'hideRecurring' in value && typeof value.hideRecurring === 'boolean'
        && 'hideTimed' in value && typeof value.hideTimed === 'boolean';
}

export function isEventVisible(event: EventInstance, visibility: EventVisibility): boolean {
    if (visibility.hideRecurring && event.recurring) return false;
    return !(visibility.hideTimed && event.timing.kind === 'timed');
}
