import type {
    AppEventFilter,
    AppEventFilterColumn,
    AppTaskFilter,
    AppTaskFilterColumn,
    FilterGroup,
    FilterNode,
    FilterOperator,
    FilterType,
    LogicalOperator,
    Value as JsonValue,
} from '../bindings/index.generated';

export interface FilterCondition<C extends string = string> {
    column: C | null;
    operator: FilterOperator | null;
    value: JsonValue | null;
}

export interface FilterColumn<C extends string = string> {
    id: C;
    label: string;
    filterType?: FilterType;
}

export interface FilterOption {
    label: string;
    value: string;
}

export function emptyFilterGroup<F>(): FilterGroup<F> {
    return { operator: 'and', children: [] };
}

export function filterGroup<F>(filters: F[], operator: LogicalOperator = 'and'): FilterGroup<F> {
    return { operator, children: filters.map(filter => ({ kind: 'condition', filter })) };
}

export function countFilters<F>(group: FilterGroup<F>): number {
    return group.children.reduce((count, child) => count + (child.kind === 'condition' ? 1 : countFilters(child)), 0);
}

const VALUE_FREE_OPERATORS = ['is empty', 'is not empty'] as const;
const EQUALITY_OPERATORS = ['is', 'is not', ...VALUE_FREE_OPERATORS] as const;
const DATE_OPERATORS = ['is', 'is not', 'before', 'after', 'on or before', 'on or after', ...VALUE_FREE_OPERATORS] as const;
const ALL_OPERATORS = [...DATE_OPERATORS, 'contains', 'does not contain'] as const;

export function isValueFreeOperator(operator: string | null | undefined): boolean {
    return VALUE_FREE_OPERATORS.some(candidate => candidate === operator);
}

export function operatorsForColumn(column?: FilterColumn): readonly FilterOperator[] {
    return column?.filterType === 'Date' ? DATE_OPERATORS : EQUALITY_OPERATORS;
}

export function newFilterCondition<C extends string>(column: FilterColumn<C>): FilterCondition<C> {
    return { column: column.id, operator: 'is', value: [] };
}

export function filterStrings(value: unknown): string[] {
    const values: unknown[] = Array.isArray(value) ? value : [value];
    return values.filter((item): item is string | number => typeof item === 'string' || typeof item === 'number').map(String);
}

function isLogicalOperator(value: unknown): value is LogicalOperator {
    return value === 'and' || value === 'or';
}

function isFilterOperator(value: unknown): value is FilterOperator {
    return ALL_OPERATORS.some(operator => operator === value);
}

function isCondition<C extends string>(value: unknown, columns: readonly C[]): value is FilterCondition<C> {
    if (typeof value !== 'object' || value === null) return false;
    if (!('column' in value) || !('operator' in value) || !('value' in value)) return false;
    if (value.column !== null && !columns.some(column => column === value.column)) return false;
    return value.operator === null || isFilterOperator(value.operator);
}

function isFilterNode<F>(value: unknown, isLeaf: (value: unknown) => value is F): value is FilterNode<F> {
    if (typeof value !== 'object' || value === null || !('kind' in value)) return false;
    if (value.kind === 'condition') return 'filter' in value && isLeaf(value.filter);
    return value.kind === 'group' && isFilterGroup(value, isLeaf);
}

function isFilterGroup<F>(value: unknown, isLeaf: (value: unknown) => value is F): value is FilterGroup<F> {
    if (typeof value !== 'object' || value === null) return false;
    if (!('operator' in value) || !isLogicalOperator(value.operator)) return false;
    return 'children' in value && Array.isArray(value.children)
        && value.children.every((child: unknown) => isFilterNode(child, isLeaf));
}

const TASK_COLUMNS = ['id', 'title', 'status', 'priority', 'tags', 'start', 'due', 'added', 'remotecollectionid'] as const satisfies readonly AppTaskFilterColumn[];
const EVENT_COLUMNS = ['title', 'status', 'priority', 'tags', 'remotecollectionid'] as const satisfies readonly AppEventFilterColumn[];

export function isTaskFilterGroup(value: unknown): value is FilterGroup<AppTaskFilter> {
    return isFilterGroup(value, (leaf): leaf is AppTaskFilter => isCondition(leaf, TASK_COLUMNS));
}

export function isEventFilterGroup(value: unknown): value is FilterGroup<AppEventFilter> {
    return isFilterGroup(value, (leaf): leaf is AppEventFilter => isCondition(leaf, EVENT_COLUMNS));
}
