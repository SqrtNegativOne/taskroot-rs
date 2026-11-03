import { addDays, addMonths, startOfMonth, startOfWeek, ymd } from '../../../lib/time';
import { DateGridView, DAYS_IN_WEEK } from './constants';

export interface GridCell {
    date: Date;
    outOfMonth: boolean;
}

const VIEW_DAY_COUNTS: Record<DateGridView, number> = {
    [DateGridView.Month]: 42,
    [DateGridView.OneWeek]: DAYS_IN_WEEK,
    [DateGridView.ThreeWeeks]: DAYS_IN_WEEK * 3,
};

export function buildDateGrid(anchor: Date, view: DateGridView) {
    const isMonth = view === DateGridView.Month;
    const start = startOfWeek(isMonth ? startOfMonth(anchor) : anchor);
    const cells: GridCell[] = Array.from({ length: VIEW_DAY_COUNTS[view] }, (_, i) => {
        const date = addDays(start, i);
        return { date, outOfMonth: isMonth && date.getMonth() !== anchor.getMonth() };
    });
    return {
        cells,
        range: { startDate: ymd(cells[0].date), endDate: ymd(cells[cells.length - 1].date) },
    };
}

export function shiftDateGridAnchor(anchor: Date, view: DateGridView, step: number): Date {
    if (view === DateGridView.Month) return addMonths(anchor, step);
    return addDays(anchor, VIEW_DAY_COUNTS[view] * step);
}
