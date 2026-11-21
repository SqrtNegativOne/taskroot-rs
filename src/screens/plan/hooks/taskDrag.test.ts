import { describe, expect, it } from 'vitest';
import { taskDropFor } from './taskDrag.svelte';

describe('taskDropFor', () => {
    it('assigns the start date when dropped on a date-grid cell', () => {
        expect(taskDropFor({ kind: 'grid-day', date: '2026-09-10' }, 30)).toEqual({
            kind: 'start',
            date: '2026-09-10',
        });
    });

    it('schedules a linked event at the dropped timeline minute', () => {
        expect(taskDropFor({ kind: 'day-time', date: '2026-09-10', minute: 570 }, 45)).toEqual({
            kind: 'schedule',
            slot: { date: '2026-09-10', minute: 570, duration: 45 },
        });
    });

    it('ignores targets without a date', () => {
        expect(taskDropFor({ kind: 'grid-day' }, 30)).toBeUndefined();
    });
});
