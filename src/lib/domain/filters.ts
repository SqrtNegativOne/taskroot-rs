import type { AppTaskStatus } from '../bindings/AppTaskStatus.generated';
import type { TaskPriority } from '../bindings/TaskPriority.generated';
import type { HydratedEvent } from './events';

export interface FilterLike {
    readonly column?: string | null;
    readonly operator?: string | null;
    readonly value?: unknown;
}

export interface FilterDefaults {
    status?: AppTaskStatus;
    priority?: TaskPriority;
    tags?: string[];
    [key: string]: unknown;
}

const FALLBACKS: Record<string, readonly (string | number)[]> = {
    status: ['todo', 'next-up', 'doing', 'done'],
    priority: [1, 2, 3, 4, 0],
};

function normalizeFilterValues(raw: unknown): readonly (string | number)[] {
    if (raw === undefined || raw === null || raw === '') {
        return [];
    }
    const arr = Array.isArray(raw) ? raw : [raw];
    const result: (string | number)[] = [];
    for (const item of arr) {
        if (typeof item === 'string' || typeof item === 'number') {
            result.push(item);
        } else if (typeof item === 'object' && item !== null && 'name' in item) {
            result.push(String((item as { readonly name: unknown }).name));
        }
    }
    return result;
}

function processFilterItem(
    f: FilterLike,
    req: Record<string, Set<string | number>>,
    excl: Record<string, Set<string | number>>,
) {
    if (!f.column) return;
    const values = normalizeFilterValues(f.value);
    if (values.length === 0) return;

    const col = f.column === 'tag' ? 'tags' : f.column;
    const isExclusion = f.operator === 'is not' || f.operator === 'does not contain';
    const targetMap = isExclusion ? excl : req;

    const set = targetMap[col] ?? new Set<string | number>();
    targetMap[col] = set;
    for (const val of values) {
        set.add(val);
    }
}

function processSingleValueCol(
    col: string,
    reqCol: Set<string | number> | undefined,
    exclCol: Set<string | number> | undefined,
    defaults: FilterDefaults,
) {
    if (reqCol?.size === 1) {
        const val = Array.from(reqCol)[0];
        if (val !== undefined && (!exclCol || !exclCol.has(val))) {
            defaults[col] = val as never;
        }
        return;
    }

    const fallbacks = FALLBACKS[col];
    if (!exclCol || !fallbacks) {
        return;
    }

    for (const fallback of fallbacks) {
        if (!exclCol.has(fallback)) {
            defaults[col] = fallback as never;
            return;
        }
    }
}

export function computeFilterDefaults(filters: readonly FilterLike[] = []): FilterDefaults {
    const req: Record<string, Set<string | number>> = {};
    const excl: Record<string, Set<string | number>> = {};

    for (const f of filters) {
        processFilterItem(f, req, excl);
    }

    const defaults: FilterDefaults = {};
    const singleValueCols = ['status', 'priority'];
    for (const col of singleValueCols) {
        processSingleValueCol(col, req[col], excl[col], defaults);
    }

    const reqTags = req['tags'];
    if (reqTags) {
        const exclTags = excl['tags'];
        const validTags = Array.from(reqTags)
            .filter((t) => !exclTags || !exclTags.has(t))
            .map(String);
        if (validTags.length > 0) {
            defaults.tags = validTags;
        }
    }

    return defaults;
}

function matchesEventTag(e: HydratedEvent, values: readonly string[]): boolean {
    const taskTags = e.task?.tags ?? [];
    const allTags = new Set(
        taskTags.map((t) => {
            const raw: unknown = t;
            if (typeof raw === 'string') return raw.toLowerCase();
            if (typeof raw === 'object' && raw !== null && 'name' in raw) {
                return String((raw as { readonly name: unknown }).name).toLowerCase();
            }
            return '';
        }),
    );
    return values.some((v) => allTags.has(v.toLowerCase()));
}

function matchesTaskStatus(e: HydratedEvent, values: readonly string[]): boolean {
    return values.some((v) => {
        if (v === 'none') return !e.task;
        if (v === 'done') return e.task?.status === 'done';
        if (v === 'todo') return e.task?.status !== 'done';
        return e.task?.status === v;
    });
}

function matchesFilter(e: HydratedEvent, f: FilterLike): boolean {
    const rawValues = Array.isArray(f.value) ? f.value : [f.value];
    const values = rawValues.map(String);
    if (values.length === 0) return true;

    let match = true;
    if (f.column === 'type') {
        const eventType = (e as { readonly type?: string }).type;
        match = eventType !== undefined && values.includes(eventType);
    } else if (f.column === 'tag' || f.column === 'tags') {
        match = matchesEventTag(e, values);
    } else if (f.column === 'taskStatus' || f.column === 'status') {
        match = matchesTaskStatus(e, values);
    } else if (f.column === 'category') {
        match = values.includes(e.category ?? '');
    }

    const isExclusion = f.operator === 'is not' || f.operator === 'does not contain';
    return isExclusion ? !match : match;
}

export function filterEvents(
    evs: readonly HydratedEvent[],
    filter?: readonly FilterLike[],
): HydratedEvent[] {
    if (!filter || filter.length === 0) return [...evs];

    let filtered = [...evs];
    for (const f of filter) {
        if (!f.column || f.value === undefined || f.value === null || f.value === '') continue;
        filtered = filtered.filter((e) => matchesFilter(e, f));
    }
    return filtered;
}

export function sortEvents(
    evs: readonly HydratedEvent[],
    sort?: string,
): HydratedEvent[] {
    if (!sort) return [...evs];
    const sorted = [...evs];
    sorted.sort((a, b) => {
        if (sort === 'taskStatus') {
            const aDone = a.task?.status === 'done' ? 1 : 0;
            const bDone = b.task?.status === 'done' ? 1 : 0;
            if (aDone !== bDone) return aDone - bDone;
        }
        return (a.startTime || '').localeCompare(b.startTime || '');
    });
    return sorted;
}
