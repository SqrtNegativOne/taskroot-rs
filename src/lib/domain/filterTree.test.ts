import { describe, expect, it } from 'vitest';
import { computeFilterDefaults, emptyFilterGroup, filterGroup, isEventFilterGroup, isTaskFilterGroup, countFilters } from '../domain';
import type { AppTaskFilter, FilterGroup, FilterNode } from '../domain';

const condition = (filter: AppTaskFilter): FilterNode<AppTaskFilter> => ({ kind: 'condition', filter });
const status = (value: string): FilterNode<AppTaskFilter> => condition({ column: 'status', operator: 'is', value: [value] });
const priority = (value: string): FilterNode<AppTaskFilter> => condition({ column: 'priority', operator: 'is', value: [value] });

const nested: FilterGroup<AppTaskFilter> = {
    operator: 'and', children: [
        condition({ column: 'tags', operator: 'is', value: ['work'] }),
        { kind: 'group', operator: 'or', children: [status('todo'), status('next-up')] },
    ],
};

describe('recursive filter state', () => {
    it('restores nested trees and counts conditions, not groups', () => {
        const restored: unknown = JSON.parse(JSON.stringify(nested));
        expect(isTaskFilterGroup(restored)).toBe(true);
        expect(countFilters(nested)).toBe(3);
    });

    it('restores value-free empty-field conditions', () => {
        expect(isTaskFilterGroup(filterGroup([
            { column: 'start', operator: 'is empty', value: [] },
            { column: 'tags', operator: 'is not empty', value: [] },
        ]))).toBe(true);
        expect(isEventFilterGroup(filterGroup([
            { column: 'tags', operator: 'is empty', value: [] },
        ]))).toBe(true);
    });

    it('accepts draft conditions and empty groups', () => {
        expect(isTaskFilterGroup({ operator: 'or', children: [
            condition({ column: null, operator: null, value: null }),
            { kind: 'group', operator: 'and', children: [] },
        ] })).toBe(true);
    });

    it.each([
        [],
        [{ column: 'status', operator: 'is', value: ['todo'] }],
        { operator: 'xor', children: [] },
        { operator: 'and', children: [{ kind: 'group', operator: 'or', children: 'invalid' }] },
        { operator: 'and', children: [{ kind: 'condition', filter: { column: 'status', operator: 'equals', value: [] } }] },
        { operator: 'and', children: [{ kind: 'condition', filter: { column: 'status', operator: 'is' } }] },
        { operator: 'and', children: [{ kind: 'unknown', children: [] }] },
    ])('rejects malformed or obsolete filter state: %j', value => {
        expect(isTaskFilterGroup(value)).toBe(false);
        expect(isEventFilterGroup(value)).toBe(false);
    });

    it('rejects task-only fields in event filters at any depth', () => {
        const group = { operator: 'or', children: [{ kind: 'group', ...filterGroup([{ column: 'due', operator: 'before', value: ['today'] }]) }] };
        expect(isTaskFilterGroup(group)).toBe(true);
        expect(isEventFilterGroup(group)).toBe(false);
    });
});

describe('new-task defaults from nested filters', () => {
    it('uses unconditional constraints but does not combine alternative statuses', () => {
        expect(computeFilterDefaults(nested)).toEqual({ tags: ['work'] });
    });

    it('infers shared constraints across OR branches without merging incompatible choices', () => {
        const group: FilterGroup<AppTaskFilter> = { operator: 'or', children: [
            { kind: 'group', operator: 'and', children: [status('todo'), priority('1')] },
            { kind: 'group', operator: 'and', children: [status('todo'), priority('4')] },
        ] };
        expect(computeFilterDefaults(group)).toEqual({ status: 'todo' });
    });

    it('does not let a nested ambiguous OR erase a surrounding required condition', () => {
        const group: FilterGroup<AppTaskFilter> = { operator: 'and', children: [priority('3'), { kind: 'group', operator: 'or', children: [status('todo'), status('done')] }] };
        expect(computeFilterDefaults(group)).toEqual({ priority: 3 });
    });

    it('ignores unfinished OR branches when inferring defaults', () => {
        const group: FilterGroup<AppTaskFilter> = { operator: 'or', children: [status('next-up'), { kind: 'group', ...emptyFilterGroup<AppTaskFilter>() }, condition({ column: 'status', operator: 'is', value: [''] })] };
        expect(computeFilterDefaults(group)).toEqual({ status: 'next-up' });
    });

    it('does not infer a value from an OR alternative to a value-free condition', () => {
        expect(computeFilterDefaults(filterGroup([
            { column: 'tags', operator: 'is empty', value: [] },
            { column: 'tags', operator: 'is', value: ['work'] },
        ], 'or'))).toEqual({});
        expect(computeFilterDefaults(filterGroup([
            { column: 'start', operator: 'is not empty', value: [] },
            { column: 'priority', operator: 'is', value: ['3'] },
        ], 'or'))).toEqual({});
    });

    it('ignores stale unary values while retaining surrounding required defaults', () => {
        expect(computeFilterDefaults(filterGroup([
            { column: 'tags', operator: 'is empty', value: ['stale'] },
            { column: 'priority', operator: 'is not empty', value: ['4'] },
            { column: 'status', operator: 'is', value: ['todo'] },
        ]))).toEqual({ status: 'todo' });
    });

    it('normalizes numeric string priorities to the typed task value', () => {
        expect(computeFilterDefaults(filterGroup([{ column: 'priority', operator: 'is', value: ['3'] }]))).toEqual({ priority: 3 });
    });

    it('does not produce invalid statuses or priorities', () => {
        expect(computeFilterDefaults(filterGroup([
            { column: 'status', operator: 'is', value: ['bogus'] },
            { column: 'priority', operator: 'is', value: ['99'] },
        ]))).toEqual({});
    });
});
