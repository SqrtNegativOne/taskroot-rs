import type { AppEvent } from '../../../lib/domain';

export interface DayLayoutEvent {
    event: AppEvent;
    startMins: number;
    endMins: number;
}

// Simple overlap layout: assign each event to the earliest lane that's free.
export function layoutEvents(events: DayLayoutEvent[]) {
    const placed: { start: number; end: number; lane: number }[] = [];
    const result: { event: AppEvent; startMins: number; endMins: number; lane: number; lanes?: number }[] = [];
    
    for (const ev of events) {
        let lane = 0;
        while (
            placed.some(
                (p) =>
                    p.lane === lane &&
                    !(p.end <= ev.startMins || p.start >= ev.endMins),
            )
        ) {
            lane++;
        }
        placed.push({ start: ev.startMins, end: ev.endMins, lane });
        result.push({ event: ev.event, startMins: ev.startMins, endMins: ev.endMins, lane });
    }
    
    return result.map((r) => {
        let maxLane = r.lane;
        for (const p of placed) {
            if (!(p.end <= r.startMins || p.start >= r.endMins)) {
                if (p.lane > maxLane) maxLane = p.lane;
            }
        }
        return Object.assign({}, r, { lanes: maxLane + 1 });
    });
}
