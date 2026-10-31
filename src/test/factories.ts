import type { AppEvent, EventInstance, EventTiming } from '../lib/domain';

type TimedTiming = Extract<EventTiming, { kind: 'timed' }>;

export interface TimedTimingOverrides {
    kind?: 'timed';
    start?: string;
    end?: string;
    timezone?: string | null;
}

export interface AllDayTimingOverrides {
    kind: 'allDay';
    startDate?: string;
    endDateExclusive?: string;
}

export type TimingOverrides = TimedTimingOverrides | AllDayTimingOverrides;

export type InstanceOverrides = Omit<Partial<EventInstance>, 'timing'> & {
    timing?: TimingOverrides;
};

function defaultTimedTiming(): TimedTiming {
    const start = new Date(2026, 8, 7, 9, 0);
    const end = new Date(2026, 8, 7, 10, 0);
    return {
        kind: 'timed',
        start: start.toISOString(),
        end: end.toISOString(),
        timezone: null,
    };
}

function isAllDayTiming(
    overrides: TimingOverrides | undefined,
): overrides is AllDayTimingOverrides {
    return overrides?.kind === 'allDay';
}

function resolveTiming(overrides: TimingOverrides | undefined): EventTiming {
    if (isAllDayTiming(overrides)) {
        return {
            kind: 'allDay',
            startDate: overrides.startDate ?? '2026-09-07',
            endDateExclusive: overrides.endDateExclusive ?? '2026-09-08',
        };
    }

    const fallback = defaultTimedTiming();
    if (!overrides) {
        return fallback;
    }

    return {
        kind: 'timed',
        start: overrides.start ?? fallback.start,
        end: overrides.end ?? fallback.end,
        timezone: overrides.timezone === undefined ? fallback.timezone : overrides.timezone,
    };
}

function occurrenceStartFor(timing: EventTiming): string {
    return timing.kind === 'allDay' ? timing.startDate : timing.start;
}

export function makeInstance(overrides: InstanceOverrides = {}): EventInstance {
    const { timing: timingOverrides, ...rest } = overrides;
    const timing = resolveTiming(timingOverrides);
    const id = rest.id ?? 'event-1';
    const occurrenceStart = rest.occurrenceStart ?? occurrenceStartFor(timing);

    return {
        id,
        occurrenceKey: `${id}@${occurrenceStart}`,
        occurrenceStart,
        title: 'Event',
        recurring: false,
        status: 'confirmed',
        ...rest,
        timing,
    };
}

export function makeEvent(overrides: Partial<AppEvent> = {}): AppEvent {
    return {
        id: 'event-1',
        title: 'Event',
        startTime: '2026-09-07T09:00:00',
        endTime: '2026-09-07T10:00:00',
        ...overrides,
    };
}
