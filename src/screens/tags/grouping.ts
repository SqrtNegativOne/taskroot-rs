import type { AppEvent, AppTask } from '../../lib/domain';

export interface TagGroup {
    name: string;
    tasks: AppTask[];
    events: AppEvent[];
}

function tagNames(tags: AppTask['tags']): string[] {
    if (!tags) return [];
    return [...new Set(tags.map((tag) => tag.name))];
}

function compareTagGroups(a: TagGroup, b: TagGroup): number {
    const lowerA = a.name.toLowerCase();
    const lowerB = b.name.toLowerCase();
    if (lowerA !== lowerB) return lowerA < lowerB ? -1 : 1;
    if (a.name === b.name) return 0;
    return a.name < b.name ? -1 : 1;
}

/**
 * Group every task and event under each of its tag names, sorted
 * case-insensitively so near-duplicate spellings sit next to each other.
 * Different spellings stay separate groups so they can be audited.
 */
export function groupByTag(
    tasks: readonly AppTask[],
    events: readonly AppEvent[],
): TagGroup[] {
    const groups = new Map<string, TagGroup>();

    function groupFor(name: string): TagGroup {
        const existing = groups.get(name);
        if (existing) return existing;
        const group: TagGroup = { name, tasks: [], events: [] };
        groups.set(name, group);
        return group;
    }

    for (const task of tasks) {
        for (const name of tagNames(task.tags)) {
            groupFor(name).tasks.push(task);
        }
    }

    for (const event of events) {
        for (const name of tagNames(event.tags)) {
            groupFor(name).events.push(event);
        }
    }

    return [...groups.values()].sort(compareTagGroups);
}
