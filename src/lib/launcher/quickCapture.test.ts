import { describe, expect, it } from 'vitest';
import type { AppCalendar } from '../domain';
import { parseQuickCapture } from './quickCapture';

function cal(
    id: string,
    summary: string,
    component: 'VTODO' | 'VEVENT' = 'VTODO',
    writable = true,
): AppCalendar {
    return { id, summary, component, writable };
}

const SAMPLE_CALENDARS: AppCalendar[] = [
    cal('/cal/tasks/', 'Tasks'),
    cal('/cal/work/', 'Work'),
    cal('/cal/work-extra/', 'Work Extra'),
    cal('/cal/dup-a/', 'Dup'),
    cal('/cal/dup-b/', 'Dup'),
    cal('/cal/readonly/', 'Read Only', 'VTODO', false),
    cal('/cal/events/', 'Events', 'VEVENT', true),
];

// Thursday, November 19, 2026 at 10:00:00 local time
const THU_MORNING = new Date(2026, 10, 19, 10, 0, 0);
// Wednesday, November 25, 2026 at 10:00:00 local time
const WED_MORNING = new Date(2026, 10, 25, 10, 0, 0);

describe('parseQuickCapture', () => {
    describe('natural-language dates and times', () => {
        it('parses combined weekday, time, and sigils cleanly', () => {
            const preview = parseQuickCapture(
                'Finish PCC next wed 3pm #work p1 +work',
                THU_MORNING,
                SAMPLE_CALENDARS,
            );

            expect(preview.title).toBe('Finish PCC');
            expect(preview.due).toBe('2026-11-25T15:00:00');
            expect(preview.hasTime).toBe(true);
            expect(preview.tags).toEqual(['work']);
            expect(preview.priority).toBe(4);
            expect(preview.list?.summary).toBe('Work');
            expect(preview.listQuery).toBe('work');
            expect(preview.listError).toBeNull();
        });

        it('handles next/this/bare weekday when reference day is the same weekday', () => {
            expect(parseQuickCapture('Sync next wed', WED_MORNING, []).due).toBe('2026-12-02');
            expect(parseQuickCapture('Sync this wed', WED_MORNING, []).due).toBe('2026-11-25');
            expect(parseQuickCapture('Sync wed', WED_MORNING, []).due).toBe('2026-11-25');
        });

        it.each<[string, string, string]>([
            ['Write report today', 'Write report', '2026-11-19'],
            ['Write report tomorrow', 'Write report', '2026-11-20'],
            ['Pay rent in 3 days', 'Pay rent', '2026-11-22'],
            ['File taxes Dec 15', 'File taxes', '2026-12-15'],
            ['End of month review', 'review', '2026-11-30'],
            ['Meeting 2026-12-01', 'Meeting', '2026-12-01'],
        ])('emits floating YYYY-MM-DD for date-only phrase: %s', (input, expectedTitle, expectedDue) => {
            const result = parseQuickCapture(input, THU_MORNING, []);
            expect(result.title).toBe(expectedTitle);
            expect(result.due).toBe(expectedDue);
            expect(result.hasTime).toBe(false);
        });

        it.each<[string, string, string]>([
            ['Standup tomorrow at 9:30am', 'Standup', '2026-11-20T09:30:00'],
            ['Review today 15:30', 'Review', '2026-11-19T15:30:00'],
            ['Call mom at 3pm', 'Call mom', '2026-11-19T15:00:00'],
            ['Call mom 12:05', 'Call mom', '2026-11-19T12:05:00'],
            ['Demo next Friday at 2:30pm', 'Demo', '2026-11-27T14:30:00'],
            ['Renew license Jan 5 10am', 'Renew license', '2027-01-05T10:00:00'],
            ['Take medicine in 2 hours', 'Take medicine', '2026-11-19T12:00:00'],
            ['Check oven in 30 minutes', 'Check oven', '2026-11-19T10:30:00'],
            ['Deploy at 15:30', 'Deploy', '2026-11-19T15:30:00'],
        ])('emits local wall-clock timestamp for timed phrase: %s', (input, expectedTitle, expectedDue) => {
            const result = parseQuickCapture(input, THU_MORNING, []);
            expect(result.title).toBe(expectedTitle);
            expect(result.due).toBe(expectedDue);
            expect(result.hasTime).toBe(true);
        });

        it('strips dangling prepositions immediately preceding a matched date', () => {
            expect(parseQuickCapture('Submit report by Friday', THU_MORNING, [])).toMatchObject({
                title: 'Submit report',
                due: '2026-11-20',
                hasTime: false,
            });
            expect(parseQuickCapture('Finish doc due tomorrow', THU_MORNING, [])).toMatchObject({
                title: 'Finish doc',
                due: '2026-11-20',
                hasTime: false,
            });
            expect(parseQuickCapture('File on Dec 15', THU_MORNING, [])).toMatchObject({
                title: 'File',
                due: '2026-12-15',
                hasTime: false,
            });
            expect(parseQuickCapture('Lunch at noon', THU_MORNING, [])).toMatchObject({
                title: 'Lunch',
                due: '2026-11-19T12:00:00',
                hasTime: true,
            });
            expect(parseQuickCapture('Call by 5pm', THU_MORNING, [])).toMatchObject({
                title: 'Call',
                due: '2026-11-19T17:00:00',
                hasTime: true,
            });
        });

        it('does not strip word suffixes that look like prepositions', () => {
            expect(parseQuickCapture('Buy oat milk tomorrow', THU_MORNING, []).title).toBe(
                'Buy oat milk',
            );
            expect(parseQuickCapture('Chat tomorrow', THU_MORNING, []).title).toBe('Chat');
        });

        it('rolls a time-only phrase forward when the time already passed today', () => {
            const lateAfternoon = new Date(2026, 10, 19, 17, 0, 0);
            const result = parseQuickCapture('Call mom at 3pm', lateAfternoon, []);

            expect(result.title).toBe('Call mom');
            expect(result.due).toBe('2026-11-20T15:00:00');
            expect(result.hasTime).toBe(true);
        });
    });

    describe('false-positive guards', () => {
        it.each([
            'Buy 3 apples',
            'Wedding planning notes',
            'Meet at 25:00',
            'Meet at 13pm',
        ])('does not invent a date for: %s', (input) => {
            const result = parseQuickCapture(input, THU_MORNING, []);
            expect(result.title).toBe(input);
            expect(result.due).toBeNull();
            expect(result.hasTime).toBe(false);
        });
    });

    describe('sigils', () => {
        it('extracts tags, priorities, durations, and list queries', () => {
            const result = parseQuickCapture(
                'Write spec #docs #q4 ^1h30m +errands p2',
                THU_MORNING,
                [],
            );

            expect(result.title).toBe('Write spec');
            expect(result.tags).toEqual(['docs', 'q4']);
            expect(result.estimateMinutes).toBe(90);
            expect(result.listQuery).toBe('errands');
            expect(result.priority).toBe(3);
        });

        it.each<[string, number]>([
            ['p1', 4],
            ['p2', 3],
            ['p3', 2],
            ['p4', 1],
            ['!0', 0],
            ['!1', 1],
            ['!2', 2],
            ['!3', 3],
            ['!4', 4],
        ])('maps priority sigil %s to %i', (token, expected) => {
            const result = parseQuickCapture(`Task ${token}`, THU_MORNING, []);
            expect(result.title).toBe('Task');
            expect(result.priority).toBe(expected);
        });

        it('parses ^m, ^h, ^hXm, and bare integer durations', () => {
            expect(parseQuickCapture('Task ^45m', THU_MORNING, []).estimateMinutes).toBe(45);
            expect(parseQuickCapture('Task ^2h', THU_MORNING, []).estimateMinutes).toBe(120);
            expect(parseQuickCapture('Task ^1h30m', THU_MORNING, []).estimateMinutes).toBe(90);
            expect(parseQuickCapture('Task ^30', THU_MORNING, []).estimateMinutes).toBe(30);
        });

        it.each([
            'Buy milk +',
            'Buy milk #',
            'Buy milk ^',
            'Buy milk ^abc',
            'Buy milk p0',
            'Buy milk p5',
            'Buy milk p10',
            'Buy milk P1',
            'Buy milk !',
            'Buy milk !5',
            'Buy milk !notanumber',
        ])('preserves invalid sigil form untouched: %s', (input) => {
            const result = parseQuickCapture(input, THU_MORNING, []);
            expect(result.title).toBe(input);
            expect(result.priority).toBeNull();
            expect(result.estimateMinutes).toBeNull();
            expect(result.listQuery).toBeNull();
            expect(result.tags).toEqual([]);
        });
    });

    describe('list resolution', () => {
        it('uses the configured default task list, falling back to the first writable VTODO', () => {
            const withDefault = parseQuickCapture(
                'Buy milk',
                THU_MORNING,
                SAMPLE_CALENDARS,
                '/cal/work/',
            );
            expect(withDefault.list?.summary).toBe('Work');
            expect(withDefault.listError).toBeNull();

            const withoutDefault = parseQuickCapture('Buy milk', THU_MORNING, SAMPLE_CALENDARS, '');
            expect(withoutDefault.list?.summary).toBe('Tasks');
            expect(withoutDefault.listError).toBeNull();

            const invalidDefault = parseQuickCapture(
                'Buy milk',
                THU_MORNING,
                SAMPLE_CALENDARS,
                '/cal/readonly/',
            );
            expect(invalidDefault.list?.summary).toBe('Tasks');
            expect(invalidDefault.listError).toBeNull();
        });

        it('resolves an exact summary match case-insensitively', () => {
            const result = parseQuickCapture('Task +work', THU_MORNING, SAMPLE_CALENDARS);
            expect(result.list?.summary).toBe('Work');
            expect(result.listError).toBeNull();
        });

        it('resolves a unique id prefix case-insensitively', () => {
            const result = parseQuickCapture('Task +/cal/TASKS', THU_MORNING, SAMPLE_CALENDARS);
            expect(result.list?.summary).toBe('Tasks');
            expect(result.listError).toBeNull();
        });

        it('reports ambiguous summary and id-prefix matches', () => {
            const dupSummary = parseQuickCapture('Task +dup', THU_MORNING, SAMPLE_CALENDARS);
            expect(dupSummary.list).toBeNull();
            expect(dupSummary.listError).toBe('Multiple task lists match "dup"');

            const dupPrefix = parseQuickCapture('Task +/cal/work', THU_MORNING, SAMPLE_CALENDARS);
            expect(dupPrefix.list).toBeNull();
            expect(dupPrefix.listError).toBe('Multiple task lists match "/cal/work"');
        });

        it.each(['missing', 'ReadOnly', 'Events', '/cal/readonly/', '/cal/events/'])(
            'rejects unknown, read-only, or VEVENT list query: %s',
            (query) => {
                const result = parseQuickCapture(`Task +${query}`, THU_MORNING, SAMPLE_CALENDARS);
                expect(result.list).toBeNull();
                expect(result.listError).toBe(`Unknown task list "${query}"`);
            },
        );
    });
});
