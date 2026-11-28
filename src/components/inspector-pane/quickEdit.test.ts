import { describe, expect, it } from 'vitest';
import type { AppCalendar, AppTask } from '../../lib/domain';
import {
    buildTaskQuickEditPatches,
    hasTrailingIncompleteSigil,
    parseTaskTitleQuickEdit,
} from './quickEdit';

function cal(
    id: string,
    summary: string,
    component: 'VTODO' | 'VEVENT' = 'VTODO',
    writable = true,
): AppCalendar {
    return { id, summary, component, writable };
}

const CALENDARS: AppCalendar[] = [
    cal('/cal/tasks/', 'Tasks'),
    cal('/cal/work/', 'Work'),
    cal('/cal/readonly/', 'Read Only', 'VTODO', false),
    cal('/cal/events/', 'Events', 'VEVENT', true),
];

// Thursday, November 19, 2026 at 10:00:00 local time
const THU_MORNING = new Date(2026, 10, 19, 10, 0, 0);

describe('parseTaskTitleQuickEdit', () => {
    it('extracts sigils and natural-language dates without defaulting to another list', () => {
        const edit = parseTaskTitleQuickEdit(
            'Buy milk tomorrow at 9:30am #errands p2 ^30m',
            'Buy milk',
            THU_MORNING,
            CALENDARS,
        );

        expect(edit.hasProperties).toBe(true);
        expect(edit.title).toBe('Buy milk');
        expect(edit.due).toBe('2026-11-20T09:30:00');
        expect(edit.hasTime).toBe(true);
        expect(edit.tags).toEqual(['errands']);
        expect(edit.priority).toBe(3);
        expect(edit.estimateMinutes).toBe(30);
        expect(edit.list).toBeNull();
        expect(edit.listError).toBeNull();
        expect(edit.previewParts).toEqual([
            'Due Tomorrow 09:30',
            '#errands',
            'P3',
            '30 min',
        ]);
    });

    it('resolves explicit +list queries and reports unknown lists', () => {
        const matched = parseTaskTitleQuickEdit(
            'Buy milk +work',
            'Buy milk',
            THU_MORNING,
            CALENDARS,
        );
        expect(matched.hasProperties).toBe(true);
        expect(matched.list?.id).toBe('/cal/work/');
        expect(matched.listError).toBeNull();
        expect(matched.previewParts).toEqual(['Work']);

        const unknown = parseTaskTitleQuickEdit(
            'Buy milk +missing',
            'Buy milk',
            THU_MORNING,
            CALENDARS,
        );
        expect(unknown.hasProperties).toBe(false);
        expect(unknown.list).toBeNull();
        expect(unknown.listError).toBe('Unknown task list "missing"');
    });

    it('preserves the current title when only quick properties are typed, but allows clearing a plain title', () => {
        const onlyProps = parseTaskTitleQuickEdit(
            'tomorrow #work p1',
            'Buy milk',
            THU_MORNING,
            CALENDARS,
        );
        expect(onlyProps.hasProperties).toBe(true);
        expect(onlyProps.title).toBe('Buy milk');

        const cleared = parseTaskTitleQuickEdit('', 'Buy milk', THU_MORNING, CALENDARS);
        expect(cleared.hasProperties).toBe(false);
        expect(cleared.title).toBe('');
    });

    it('detects trailing incomplete sigil prefixes', () => {
        expect(hasTrailingIncompleteSigil('Buy milk #')).toBe(true);
        expect(hasTrailingIncompleteSigil('Buy milk +')).toBe(true);
        expect(hasTrailingIncompleteSigil('Buy milk ^')).toBe(true);
        expect(hasTrailingIncompleteSigil('Buy milk !')).toBe(true);
        expect(hasTrailingIncompleteSigil('#')).toBe(true);
        expect(hasTrailingIncompleteSigil('Buy milk #work')).toBe(false);
        expect(hasTrailingIncompleteSigil('Buy milk # ')).toBe(false);
        expect(hasTrailingIncompleteSigil('Buy milk')).toBe(false);
    });
});

describe('buildTaskQuickEditPatches', () => {
    it('merges new tags onto existing tags, omits unchanged title, and sets timezone for timed due', () => {
        const task: AppTask = {
            id: 't1',
            title: 'Buy milk',
            tags: [{ id: 'existing-1', name: 'home' }],
            remoteCollectionId: '/cal/tasks/',
        };
        const edit = parseTaskTitleQuickEdit(
            'Buy milk tomorrow at 9:30am #home #errands #errands p1 ^45m +work',
            task.title,
            THU_MORNING,
            CALENDARS,
        );

        const patches = buildTaskQuickEditPatches(task, edit, 'Asia/Kolkata');

        expect(patches).toEqual([
            { field: 'due', value: '2026-11-20T09:30:00' },
            { field: 'timezone', value: 'Asia/Kolkata' },
            { field: 'priority', value: 4 },
            { field: 'est', value: 45 },
            {
                field: 'tags',
                value: [
                    { id: 'existing-1', name: 'home' },
                    expect.objectContaining({ name: 'errands' }),
                ],
            },
            { field: 'remoteCollectionId', value: '/cal/work/' },
        ]);
    });

    it('does not touch timezone for date-only due, supports clearing priority and estimate, and skips no-op list moves', () => {
        const task: AppTask = {
            id: 't2',
            title: 'New Task',
            tags: [{ id: 't-work', name: 'work' }],
            priority: 3,
            est: 30,
            remoteCollectionId: '/cal/tasks/',
        };
        const edit = parseTaskTitleQuickEdit(
            'Write docs tomorrow #work !0 ^0m +tasks',
            task.title,
            THU_MORNING,
            CALENDARS,
        );

        const patches = buildTaskQuickEditPatches(task, edit, 'Asia/Kolkata');

        expect(patches).toEqual([
            { field: 'title', value: 'Write docs' },
            { field: 'due', value: '2026-11-20' },
            { field: 'priority', value: 0 },
            { field: 'est', value: null },
        ]);
    });
});
