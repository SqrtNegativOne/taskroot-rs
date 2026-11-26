import { describe, expect, it } from 'vitest';
import { makeInstance } from '../../../test/factories';
import { previewColumns, resizeAllDayRange } from './spanResize';

function allDaySpan() {
    return makeInstance({
        timing: {
            kind: 'allDay',
            startDate: '2026-09-09',
            endDateExclusive: '2026-09-12',
        },
    });
}

describe('resizeAllDayRange', () => {
    it('moves the start edge without changing the end', () => {
        expect(resizeAllDayRange(allDaySpan(), 'start', '2026-09-07')).toEqual({
            startDate: '2026-09-07',
            endDateExclusive: '2026-09-12',
        });
    });

    it('moves the end edge inclusively, keeping the exclusive end date', () => {
        expect(resizeAllDayRange(allDaySpan(), 'end', '2026-09-14')).toEqual({
            startDate: '2026-09-09',
            endDateExclusive: '2026-09-15',
        });
    });

    it('never collapses the range below one day', () => {
        expect(resizeAllDayRange(allDaySpan(), 'start', '2026-09-20')).toEqual({
            startDate: '2026-09-11',
            endDateExclusive: '2026-09-12',
        });
        expect(resizeAllDayRange(allDaySpan(), 'end', '2026-09-01')).toEqual({
            startDate: '2026-09-09',
            endDateExclusive: '2026-09-10',
        });
    });

    it('refuses to resize timed events', () => {
        expect(resizeAllDayRange(makeInstance(), 'end', '2026-09-14')).toBeUndefined();
    });
});

describe('previewColumns', () => {
    it('maps an in-week range to its day columns', () => {
        expect(
            previewColumns('2026-09-07', {
                startDate: '2026-09-09',
                endDateExclusive: '2026-09-12',
            }),
        ).toEqual({ startCol: 2, endColExclusive: 5 });
    });

    it('clips a preview that reaches outside the rendered week', () => {
        expect(
            previewColumns('2026-09-07', {
                startDate: '2026-09-06',
                endDateExclusive: '2026-09-14',
            }),
        ).toEqual({ startCol: 0, endColExclusive: 7 });
    });
});
