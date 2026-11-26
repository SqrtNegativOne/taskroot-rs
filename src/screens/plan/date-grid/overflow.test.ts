import { describe, expect, it } from 'vitest';
import { chipOverflow } from './overflow';

describe('chipOverflow', () => {
    it('keeps every chip visible when they all fit', () => {
        expect(chipOverflow(3, 3)).toEqual({ visible: 3, hidden: 0 });
        expect(chipOverflow(2, 4)).toEqual({ visible: 2, hidden: 0 });
    });

    it('uses the last fitting slot for the summary and counts the rest', () => {
        expect(chipOverflow(5, 3)).toEqual({ visible: 2, hidden: 3 });
        expect(chipOverflow(4, 2)).toEqual({ visible: 1, hidden: 3 });
    });

    it('summarises everything when only one slot fits', () => {
        expect(chipOverflow(3, 1)).toEqual({ visible: 0, hidden: 3 });
    });

    it('renders every chip while the cell has not been measured', () => {
        expect(chipOverflow(4, Infinity)).toEqual({ visible: 4, hidden: 0 });
    });

    it('renders nothing for an empty cell', () => {
        expect(chipOverflow(0, 3)).toEqual({ visible: 0, hidden: 0 });
    });
});
