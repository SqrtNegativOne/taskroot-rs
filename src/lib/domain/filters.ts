import type { AppTaskStatus } from '../bindings/AppTaskStatus.generated';
import type { TaskPriority } from '../bindings/TaskPriority.generated';

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
