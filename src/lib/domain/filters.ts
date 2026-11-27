import type { AppTaskStatus, FilterGroup, TaskPriority } from '../bindings/index.generated';
import { emptyFilterGroup, isValueFreeOperator } from './filterTree';
import { isAppTaskStatus } from './models';

export interface FilterLike {
    readonly column?: string | null;
    readonly operator?: string | null;
    readonly value?: unknown;
}

export interface FilterDefaults {
    status?: AppTaskStatus;
    priority?: TaskPriority;
    tags?: string[];
}

type FilterValue = string | number;

function normalizeFilterValues(raw: unknown): FilterValue[] {
    const values: unknown[] = Array.isArray(raw) ? raw : [raw];
    return values.filter((value): value is FilterValue => (typeof value === 'string' && value !== '') || typeof value === 'number');
}

function filterKey(filter: FilterLike): string {
    return JSON.stringify([filter.column, filter.operator ?? 'is', normalizeFilterValues(filter.value).map(String).sort()]);
}

function requiredConditions(group: FilterGroup<FilterLike>): FilterLike[] | undefined {
    const branches = group.children.map(child => {
        if (child.kind === 'group') return requiredConditions(child);
        const effective = child.filter.column
            && (isValueFreeOperator(child.filter.operator) || normalizeFilterValues(child.filter.value).length > 0);
        return effective ? [child.filter] : undefined;
    }).filter((branch): branch is FilterLike[] => branch !== undefined);
    const first = branches[0];
    if (!first) return undefined;
    if (group.operator === 'and') return branches.flat();
    return first.filter(filter => branches.every(branch => branch.some(candidate => filterKey(candidate) === filterKey(filter))));
}

function parsePriority(value: FilterValue | undefined): TaskPriority | undefined {
    if (typeof value === 'string' && !/^[0-4]$/.test(value)) return undefined;
    const priority = typeof value === 'string' ? Number(value) : value;
    if (priority === 0 || priority === 1 || priority === 2 || priority === 3 || priority === 4) return priority;
    return undefined;
}

function singleValue(
    required: Set<FilterValue> | undefined,
    excluded: Set<FilterValue> | undefined,
    fallbacks: readonly FilterValue[],
): FilterValue | undefined {
    if (required) {
        const value = required.size === 1 ? [...required][0] : undefined;
        return value !== undefined && !excluded?.has(value) ? value : undefined;
    }
    return excluded ? fallbacks.find(value => !excluded.has(value)) : undefined;
}

function collectConstraints(group: FilterGroup<FilterLike>) {
    const required = new Map<string, Set<FilterValue>>();
    const excluded = new Map<string, Set<FilterValue>>();
    for (const filter of requiredConditions(group) ?? []) {
        if (!filter.column || isValueFreeOperator(filter.operator)) continue;
        const values = normalizeFilterValues(filter.value);
        const target = filter.operator === 'is not' || filter.operator === 'does not contain' ? excluded : required;
        const normalized = filter.column === 'priority'
            ? values.map(parsePriority).filter((value): value is TaskPriority => value !== undefined)
            : values;
        target.set(filter.column, new Set([...(target.get(filter.column) ?? []), ...normalized]));
    }
    return { required, excluded };
}

/** Only infer constraints required by every OR branch, never flatten alternatives. */
export function computeFilterDefaults(group: FilterGroup<FilterLike> = emptyFilterGroup()): FilterDefaults {
    const { required, excluded } = collectConstraints(group);
    const defaults: FilterDefaults = {};
    const status = singleValue(required.get('status'), excluded.get('status'), ['todo', 'next-up', 'done', 'cancelled']);
    if (isAppTaskStatus(status)) defaults.status = status;
    const priority = parsePriority(singleValue(required.get('priority'), excluded.get('priority'), [1, 2, 3, 4, 0]));
    if (priority !== undefined) defaults.priority = priority;
    const tags = [...required.get('tags') ?? []].filter(tag => !excluded.get('tags')?.has(tag)).map(String);
    if (tags.length > 0) defaults.tags = tags;
    return defaults;
}
