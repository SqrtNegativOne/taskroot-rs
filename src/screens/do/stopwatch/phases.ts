export type ClockPhase = 'work' | 'break' | 'long break';

const BREAK_UNTIL_MINUTE = 5;
const WORK_UNTIL_MINUTE = 30;
const BREAK_UNTIL_MINUTE_2 = 35;
const LONG_BREAK_UNTIL_MINUTE = 35;
const HOUR_MINUTES = 60;
const MS_PER_MINUTE = 60_000;

/**
 * The guzey clock's work/break schedule: breaks occupy the first and thirty-first
 * five-minute windows of each half hour. When `haveLongBreaks` is on, the whole
 * first 35 minutes of every third hour are a long break.
 */
export function guzeyPhase(hour: number, minute: number, haveLongBreaks: boolean): ClockPhase {
    if (haveLongBreaks && hour % 3 === 0 && minute < LONG_BREAK_UNTIL_MINUTE) {
        return 'long break';
    }
    if (minute < BREAK_UNTIL_MINUTE || (minute >= WORK_UNTIL_MINUTE && minute < BREAK_UNTIL_MINUTE_2)) {
        return 'break';
    }
    return 'work';
}

/** Milliseconds left until the current guzey phase ends. */
export function guzeyRemainingMs(
    hour: number,
    minute: number,
    second: number,
    millisecond: number,
    haveLongBreaks: boolean,
): number {
    let targetMinute = HOUR_MINUTES;
    if (haveLongBreaks && hour % 3 === 0 && minute < LONG_BREAK_UNTIL_MINUTE) {
        targetMinute = LONG_BREAK_UNTIL_MINUTE;
    } else if (minute < BREAK_UNTIL_MINUTE) {
        targetMinute = BREAK_UNTIL_MINUTE;
    } else if (minute < WORK_UNTIL_MINUTE) {
        targetMinute = WORK_UNTIL_MINUTE;
    } else if (minute < BREAK_UNTIL_MINUTE_2) {
        targetMinute = BREAK_UNTIL_MINUTE_2;
    }

    const elapsed = minute * MS_PER_MINUTE + second * 1_000 + millisecond;
    return Math.max(0, targetMinute * MS_PER_MINUTE - elapsed);
}
