import { describe, expect, it } from 'vitest';
import type { EventInstance } from '../../../lib/domain';
import { addDays } from '../../../lib/time';
import { isMultiDayAllDay, layoutWeekSpans } from './spanLayout';

function allDay(id: string, startDate: string, endDateExclusive: string): EventInstance {
    return {
        id,
        occurrenceKey: `${id}@${startDate}`,
        title: 'Event',
        timing: { kind: 'allDay', startDate, endDateExclusive },
        recurring: false,
        status: 'confirmed',
    };
}

function timed(id: string, start: Date, end: Date): EventInstance {
    return {
        id,
        occurrenceKey: `${id}@${start.toISOString()}`,
        title: 'Event',
        timing: { kind: 'timed', start: start.toISOString(), end: end.toISOString(), timezone: null },
        recurring: false,
        status: 'confirmed',
    };
}

/** The seven days of the week starting on `startDate` (a Monday). */
function week(startDate: string): Date[] {
    const start = new Date(`${startDate}T00:00:00`);
    return Array.from({ length: 7 }, (_, i) => addDays(start, i));
}

describe('isMultiDayAllDay', () => {
    it('only accepts all-day ranges longer than one day', () => {
        expect(isMultiDayAllDay(allDay('a', '2026-09-07', '2026-09-08'))).toBe(false);
        expect(isMultiDayAllDay(allDay('b', '2026-09-07', '2026-09-09'))).toBe(true);
        expect(isMultiDayAllDay(timed('c', new Date(2026, 8, 7), new Date(2026, 8, 9)))).toBe(false);
    });
});

describe('layoutWeekSpans', () => {
    it('ignores single-day all-day and timed instances', () => {
        const layout = layoutWeekSpans(
            [
                allDay('one', '2026-09-09', '2026-09-10'),
                timed('t', new Date(2026, 8, 9, 9), new Date(2026, 8, 9, 10)),
            ],
            week('2026-09-07'),
        );

        expect(layout).toEqual({ spans: [], laneCount: 0 });
    });

    it('places a range inside the week on its day columns', () => {
        const layout = layoutWeekSpans([allDay('a', '2026-09-09', '2026-09-12')], week('2026-09-07'));

        expect(layout.spans).toHaveLength(1);
        const span = layout.spans[0];
        expect(span.startCol).toBe(2);
        expect(span.endColExclusive).toBe(5);
        expect(span.continuesBefore).toBe(false);
        expect(span.continuesAfter).toBe(false);
        expect(layout.laneCount).toBe(1);
    });

    it('clips a range that started the week before', () => {
        const layout = layoutWeekSpans([allDay('a', '2026-09-04', '2026-09-09')], week('2026-09-07'));

        const span = layout.spans[0];
        expect(span.startCol).toBe(0);
        expect(span.endColExclusive).toBe(2);
        expect(span.continuesBefore).toBe(true);
        expect(span.continuesAfter).toBe(false);
    });

    it('clips a range that continues into the next week', () => {
        const layout = layoutWeekSpans([allDay('a', '2026-09-11', '2026-09-15')], week('2026-09-07'));

        const span = layout.spans[0];
        expect(span.startCol).toBe(4);
        expect(span.endColExclusive).toBe(7);
        expect(span.continuesBefore).toBe(false);
        expect(span.continuesAfter).toBe(true);
    });

    it('flags both ends when the range covers the whole week', () => {
        const layout = layoutWeekSpans([allDay('a', '2026-09-01', '2026-09-20')], week('2026-09-07'));

        const span = layout.spans[0];
        expect(span.startCol).toBe(0);
        expect(span.endColExclusive).toBe(7);
        expect(span.continuesBefore).toBe(true);
        expect(span.continuesAfter).toBe(true);
    });

    it('skips ranges that do not overlap the week', () => {
        const layout = layoutWeekSpans([allDay('a', '2026-10-01', '2026-10-05')], week('2026-09-07'));

        expect(layout.spans).toEqual([]);
    });

    it('packs overlapping ranges into separate lanes and reuses freed lanes', () => {
        const layout = layoutWeekSpans(
            [
                allDay('a', '2026-09-07', '2026-09-10'),
                allDay('b', '2026-09-08', '2026-09-11'),
                allDay('c', '2026-09-11', '2026-09-13'),
            ],
            week('2026-09-07'),
        );

        const laneOf = (id: string) => layout.spans.find((s) => s.instance.id === id)?.lane;
        expect(laneOf('a')).toBe(0);
        expect(laneOf('b')).toBe(1);
        expect(laneOf('c')).toBe(0);
        expect(layout.laneCount).toBe(2);
    });

    it('produces one continuation segment per week for a multi-week range', () => {
        const event = allDay('long', '2026-09-04', '2026-09-17');

        const first = layoutWeekSpans([event], week('2026-09-07')).spans[0];
        const second = layoutWeekSpans([event], week('2026-09-14')).spans[0];

        expect(first.continuesBefore).toBe(true);
        expect(first.continuesAfter).toBe(true);
        expect(second.continuesBefore).toBe(true);
        expect(second.continuesAfter).toBe(false);
        expect(second.startCol).toBe(0);
        expect(second.endColExclusive).toBe(3);
    });
});
