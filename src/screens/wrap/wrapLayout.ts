import { MS_PER_MINUTE } from '../../lib/time';

export const MINUTES_IN_HOUR = 60;
export const HOURS_PER_DAY = 24;
export const MINUTES_PER_DAY = HOURS_PER_DAY * MINUTES_IN_HOUR;
export const MS_PER_SECOND = 1_000;

export type WrapTimelineItemKind = 'work' | 'completed' | 'completion-pin';

export interface WrapTimelineItem {
    id: string;
    kind: WrapTimelineItemKind;
    taskId?: string;
    title: string;
    startMins: number;
    endMins: number;
    layoutStartMins: number;
    layoutEndMins: number;
    durationMs?: number;
    completedAtMins?: number;
    isDone: boolean;
    lane: number;
    lanes: number;
}

export type UnlaidWrapItem = Omit<WrapTimelineItem, 'lane' | 'lanes'>;

export function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
}

export function boundedLayoutWindow(
    startMins: number,
    endMins: number,
    minVisualMins: number,
): { layoutStartMins: number; layoutEndMins: number } {
    const span = endMins - startMins;
    if (span >= minVisualMins) {
        return { layoutStartMins: startMins, layoutEndMins: endMins };
    }
    const layoutEndMins = Math.min(MINUTES_PER_DAY, startMins + minVisualMins);
    const layoutStartMins = Math.max(0, layoutEndMins - minVisualMins);
    return { layoutStartMins, layoutEndMins };
}

const KIND_ORDER: Record<WrapTimelineItemKind, number> = {
    work: 0,
    completed: 1,
    'completion-pin': 2,
};

function compareUnlaid(a: UnlaidWrapItem, b: UnlaidWrapItem): number {
    if (a.layoutStartMins !== b.layoutStartMins) return a.layoutStartMins - b.layoutStartMins;
    if (a.layoutEndMins !== b.layoutEndMins) return a.layoutEndMins - b.layoutEndMins;
    if (a.kind !== b.kind) return KIND_ORDER[a.kind] - KIND_ORDER[b.kind];
    return a.id.localeCompare(b.id);
}

function itemsOverlap(aStart: number, aEnd: number, bStart: number, bEnd: number): boolean {
    return !(aEnd <= bStart || aStart >= bEnd);
}

export function layoutTimelineItems(rawItems: UnlaidWrapItem[]): WrapTimelineItem[] {
    const sorted = [...rawItems].sort(compareUnlaid);
    const placed: { start: number; end: number; lane: number }[] = [];
    const assigned: Omit<WrapTimelineItem, 'lanes'>[] = [];

    for (const item of sorted) {
        let lane = 0;
        while (
            placed.some(
                (p) => p.lane === lane && itemsOverlap(p.start, p.end, item.layoutStartMins, item.layoutEndMins),
            )
        ) {
            lane++;
        }
        placed.push({ start: item.layoutStartMins, end: item.layoutEndMins, lane });
        assigned.push({ ...item, lane });
    }

    return assigned.map((item) => {
        let maxLane = item.lane;
        for (const p of placed) {
            if (itemsOverlap(p.start, p.end, item.layoutStartMins, item.layoutEndMins) && p.lane > maxLane) {
                maxLane = p.lane;
            }
        }
        return { ...item, lanes: maxLane + 1 };
    });
}

export function computeStartHour(
    items: readonly UnlaidWrapItem[],
    earliestWakeUpTime?: number | null,
): number {
    const wakeMins = earliestWakeUpTime ?? 0;
    const wakeHour = Number.isFinite(wakeMins)
        ? clamp(Math.floor(wakeMins / MINUTES_IN_HOUR), 0, HOURS_PER_DAY - 1)
        : 0;
    if (items.length === 0) return wakeHour;

    let minStartMins = MINUTES_PER_DAY;
    for (const item of items) {
        if (item.layoutStartMins < minStartMins) {
            minStartMins = item.layoutStartMins;
        }
    }
    const minItemHour = clamp(Math.floor(minStartMins / MINUTES_IN_HOUR), 0, HOURS_PER_DAY - 1);
    return Math.min(wakeHour, minItemHour);
}

/** Compact human duration string (e.g. `0m`, `< 1m`, `45m`, `1h 20m`). */
export function formatWrapDuration(ms: number): string {
    if (ms <= 0) return '0m';
    if (ms < MS_PER_MINUTE) return '< 1m';
    const totalMinutes = Math.round(ms / MS_PER_MINUTE);
    const hours = Math.floor(totalMinutes / MINUTES_IN_HOUR);
    const mins = totalMinutes % MINUTES_IN_HOUR;
    if (hours === 0) return `${mins}m`;
    return mins === 0 ? `${hours}h` : `${hours}h ${mins}m`;
}
