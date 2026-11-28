import { addDays, addMonths, startOfMonth, startOfWeek, ymd } from '../../../lib/time';
import { DateGridView, DAYS_IN_WEEK } from './constants';

export interface GridCell {
    date: Date;
    outOfMonth: boolean;
}

export interface MonthKey {
    year: number;
    month: number;
}

export const VISIBLE_WEEKS: Record<DateGridView, number> = {
    [DateGridView.Month]: 6,
    [DateGridView.OneWeek]: 1,
    [DateGridView.ThreeWeeks]: 3,
};

const VIEW_DAY_COUNTS: Record<DateGridView, number> = {
    [DateGridView.Month]: VISIBLE_WEEKS[DateGridView.Month] * DAYS_IN_WEEK,
    [DateGridView.OneWeek]: VISIBLE_WEEKS[DateGridView.OneWeek] * DAYS_IN_WEEK,
    [DateGridView.ThreeWeeks]: VISIBLE_WEEKS[DateGridView.ThreeWeeks] * DAYS_IN_WEEK,
};

const WEIGHT_TIE_EPSILON = 1e-6;

export function monthKeyOf(date: Date): MonthKey {
    return { year: date.getFullYear(), month: date.getMonth() };
}

export function sameMonthKey(a: MonthKey, b: MonthKey): boolean {
    return a.year === b.year && a.month === b.month;
}

export function targetWeekForAnchor(anchor: Date, view: DateGridView): Date {
    return startOfWeek(view === DateGridView.Month ? startOfMonth(anchor) : anchor);
}

export function buildInfiniteWeeks(windowStartWeek: Date, weekCount: number): Date[][] {
    const start = startOfWeek(windowStartWeek);
    return Array.from({ length: weekCount }, (_, weekIndex) =>
        Array.from({ length: DAYS_IN_WEEK }, (_, dayIndex) =>
            addDays(start, weekIndex * DAYS_IN_WEEK + dayIndex),
        ),
    );
}

interface MonthScore {
    key: MonthKey;
    weight: number;
}

function addWeekMonthScores(
    scores: Map<string, MonthScore>,
    week: readonly Date[],
    overlap: number,
): void {
    for (const date of week) {
        const year = date.getFullYear();
        const month = date.getMonth();
        const mapKey = `${year}-${month}`;
        const existing = scores.get(mapKey);
        if (existing) {
            existing.weight += overlap;
        } else {
            scores.set(mapKey, { key: { year, month }, weight: overlap });
        }
    }
}

function pickBestMonth(scores: Map<string, MonthScore>, fallback: MonthKey): MonthKey {
    let best: MonthScore | undefined;
    for (const entry of scores.values()) {
        if (!best || entry.weight > best.weight + WEIGHT_TIE_EPSILON) {
            best = entry;
            continue;
        }
        const isTie = Math.abs(entry.weight - best.weight) <= WEIGHT_TIE_EPSILON;
        if (isTie && sameMonthKey(entry.key, fallback)) {
            best = entry;
        }
    }
    return best?.key ?? fallback;
}

/**
 * Returns the calendar month occupying the largest visible area inside the
 * vertical viewport `[scrollTop, scrollTop + viewportHeight]`. Ties keep
 * `fallback` so stopping halfway between months does not flicker.
 */
export function dominantMonthInViewport(
    weeks: readonly (readonly Date[])[],
    scrollTop: number,
    viewportHeight: number,
    weekHeight: number,
    fallback: MonthKey,
): MonthKey {
    if (weeks.length === 0 || viewportHeight <= 0 || weekHeight <= 0) return fallback;

    const viewBottom = scrollTop + viewportHeight;
    const scores = new Map<string, MonthScore>();

    for (let row = 0; row < weeks.length; row++) {
        const rowTop = row * weekHeight;
        const overlap = Math.min(rowTop + weekHeight, viewBottom) - Math.max(rowTop, scrollTop);
        if (overlap > 0) addWeekMonthScores(scores, weeks[row], overlap);
    }

    return pickBestMonth(scores, fallback);
}

export function tagOutOfMonthWeeks(
    rawWeeks: readonly (readonly Date[])[],
    isMonth: boolean,
    active: MonthKey,
): GridCell[][] {
    return rawWeeks.map((week) =>
        week.map((date) => ({
            date,
            outOfMonth: isMonth && !sameMonthKey(monthKeyOf(date), active),
        })),
    );
}

export function buildDateGrid(anchor: Date, view: DateGridView) {
    const isMonth = view === DateGridView.Month;
    const start = targetWeekForAnchor(anchor, view);
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
