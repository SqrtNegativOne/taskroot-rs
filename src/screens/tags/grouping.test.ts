import { describe, expect, it } from 'vitest';
import { groupByTag } from './grouping';
import { makeEvent } from '../../test/factories';
import type { AppTask } from '../../lib/domain';

function task(id: string, title: string, tagNames: string[]): AppTask {
    return {
        id,
        title,
        tags: tagNames.map((name) => ({ id: name, name })),
    };
}

describe('groupByTag', () => {
    it('groups tasks and events by tag name, sorted case-insensitively', () => {
        const tasks = [task('t1', 'Buy milk', ['errand']), task('t2', 'Plan', ['Work'])];
        const events = [makeEvent({ id: 'e1', tags: [{ id: 'work', name: 'work' }] })];

        const groups = groupByTag(tasks, events);

        expect(groups.map((group) => group.name)).toEqual(['errand', 'Work', 'work']);
        expect(groups[0]?.tasks.map((item) => item.id)).toEqual(['t1']);
        expect(groups[1]?.tasks.map((item) => item.id)).toEqual(['t2']);
        expect(groups[2]?.events.map((item) => item.id)).toEqual(['e1']);
    });

    it('lists an item under every tag it carries and ignores untagged items', () => {
        const tasks = [
            task('t1', 'Report', ['work', 'urgent']),
            task('t2', 'Loose', []),
        ];
        const events = [makeEvent({ id: 'e1', tags: [{ id: 'work', name: 'work' }] })];

        const groups = groupByTag(tasks, events);

        expect(groups.map((group) => group.name)).toEqual(['urgent', 'work']);
        expect(groups[1]?.tasks.map((item) => item.id)).toEqual(['t1']);
        expect(groups[1]?.events.map((item) => item.id)).toEqual(['e1']);
        expect(groups.some((group) => group.tasks.some((item) => item.id === 't2'))).toBe(false);
    });

    it('does not repeat a task that carries the same tag twice', () => {
        const groups = groupByTag(
            [task('t1', 'Report', ['work', 'work'])],
            [],
        );

        expect(groups).toHaveLength(1);
        expect(groups[0]?.tasks.map((item) => item.id)).toEqual(['t1']);
    });
});
