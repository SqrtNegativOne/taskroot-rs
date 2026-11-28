import * as chrono from 'chrono-node';
import type { AppCalendar, TaskPriority } from '../domain';
import { addDays, formatHhMm, parseLocalYmd, sameDay, ymd } from '../time';

export interface QuickCapturePreview {
    title: string;
    due: string | null;
    hasTime: boolean;
    tags: string[];
    priority: TaskPriority | null;
    estimateMinutes: number | null;
    list: AppCalendar | null;
    listQuery: string | null;
    listError: string | null;
}

interface ParsedSigils {
    cleanTitle: string;
    tags: string[];
    priority: TaskPriority | null;
    estimateMinutes: number | null;
    listQuery: string | null;
}

const PRIORITY_TOKENS = new Map<string, TaskPriority>([
    ['!0', 0],
    ['!1', 1],
    ['!2', 2],
    ['!3', 3],
    ['!4', 4],
    ['p1', 4],
    ['p2', 3],
    ['p3', 2],
    ['p4', 1],
]);

const HOURS_AND_MINUTES = /^(\d+)h(\d+)m$/;
const MINUTES_ONLY = /^(\d+)m$/;
const HOURS_ONLY = /^(\d+)h$/;
const DIGITS_ONLY = /^(\d+)$/;

const DANGLING_PREPOSITION = /(?:^|\s)(?:at|on|by|due)$/i;
const LEADING_PREPOSITION = /^(?:at|on|by|due)\b/i;

/**
 * Synchronously parse a quick-capture line using sigils (`#tag`, `p1`-`p4`,
 * `!0`-`!4`, `^duration`, `+list`), natural-language dates via `chrono-node`,
 * and target task-list resolution against the active writable `VTODO` lists.
 */
export function parseQuickCapture(
    input: string,
    referenceDate: Date,
    calendars: readonly AppCalendar[],
    defaultTaskList?: string,
): QuickCapturePreview {
    const sigils = parseSigils(input);
    const { title, due, hasTime } = extractDate(sigils.cleanTitle, referenceDate);
    const { list, listError } = resolveList(calendars, sigils.listQuery, defaultTaskList);

    return {
        title,
        due,
        hasTime,
        tags: sigils.tags,
        priority: sigils.priority,
        estimateMinutes: sigils.estimateMinutes,
        list,
        listQuery: sigils.listQuery,
        listError,
    };
}

function parseSigils(input: string): ParsedSigils {
    const trimmed = input.trim();
    if (trimmed === '') {
        return { cleanTitle: '', tags: [], priority: null, estimateMinutes: null, listQuery: null };
    }

    const titleWords: string[] = [];
    const tags: string[] = [];
    let priority: TaskPriority | null = null;
    let estimateMinutes: number | null = null;
    let listQuery: string | null = null;

    for (const word of trimmed.split(/\s+/)) {
        if (word.startsWith('#') && word.length > 1) {
            tags.push(word.slice(1));
            continue;
        }
        if (word.startsWith('+') && word.length > 1) {
            listQuery = word.slice(1);
            continue;
        }
        const parsedPriority = PRIORITY_TOKENS.get(word);
        if (parsedPriority !== undefined) {
            priority = parsedPriority;
            continue;
        }
        if (word.startsWith('^')) {
            const minutes = parseDuration(word.slice(1));
            if (minutes !== null) {
                estimateMinutes = minutes;
                continue;
            }
        }
        titleWords.push(word);
    }

    return { cleanTitle: titleWords.join(' '), tags, priority, estimateMinutes, listQuery };
}

function parseDuration(raw: string): number | null {
    const combined = HOURS_AND_MINUTES.exec(raw);
    if (combined !== null) {
        return Number(combined[1]) * 60 + Number(combined[2]);
    }
    const minutes = MINUTES_ONLY.exec(raw);
    if (minutes !== null) {
        return Number(minutes[1]);
    }
    const hours = HOURS_ONLY.exec(raw);
    if (hours !== null) {
        return Number(hours[1]) * 60;
    }
    const bare = DIGITS_ONLY.exec(raw);
    if (bare !== null) {
        return Number(bare[1]);
    }
    return null;
}

function extractDate(
    cleanTitle: string,
    referenceDate: Date,
): { title: string; due: string | null; hasTime: boolean } {
    if (cleanTitle === '') {
        return { title: '', due: null, hasTime: false };
    }

    const results = chrono.parse(cleanTitle, referenceDate, { forwardDate: true });
    const match = results[0];
    if (match === undefined) {
        return { title: cleanTitle, due: null, hasTime: false };
    }

    const hasTime = match.start.isCertain('hour');
    const date = match.start.date();
    const due = hasTime ? formatLocalWallClock(date, match.start.isCertain('second')) : ymd(date);

    return {
        title: stripDateMatch(cleanTitle, match),
        due,
        hasTime,
    };
}

function stripDateMatch(text: string, match: chrono.ParsedResult): string {
    let before = text.slice(0, match.index);
    const after = text.slice(match.index + match.text.length);

    if (/\s$/.test(before) && !LEADING_PREPOSITION.test(match.text.trimStart())) {
        before = before.trimEnd().replace(DANGLING_PREPOSITION, '');
    }

    return [before.trim(), after.trim()].filter(Boolean).join(' ').replace(/\s+/g, ' ');
}

function formatLocalWallClock(date: Date, includeSeconds: boolean): string {
    const hours = pad2(date.getHours());
    const minutes = pad2(date.getMinutes());
    const seconds = pad2(includeSeconds ? date.getSeconds() : 0);
    return `${ymd(date)}T${hours}:${minutes}:${seconds}`;
}

function pad2(value: number): string {
    return value.toString().padStart(2, '0');
}

function resolveList(
    calendars: readonly AppCalendar[],
    listQuery: string | null,
    defaultTaskList?: string,
): { list: AppCalendar | null; listError: string | null } {
    const writable = calendars.filter((cal) => cal.writable && cal.component === 'VTODO');

    if (listQuery === null) {
        const configured =
            defaultTaskList !== undefined && defaultTaskList !== ''
                ? writable.find((cal) => cal.id === defaultTaskList)
                : undefined;
        return { list: configured ?? writable[0] ?? null, listError: null };
    }

    const needle = listQuery.toLowerCase();
    const exact = writable.filter((cal) => cal.summary.toLowerCase() === needle);
    if (exact.length === 1) return { list: exact[0], listError: null };
    if (exact.length > 1) return { list: null, listError: ambiguousMessage(listQuery) };

    const prefix = writable.filter((cal) => cal.id.toLowerCase().startsWith(needle));
    if (prefix.length === 1) return { list: prefix[0], listError: null };
    if (prefix.length > 1) return { list: null, listError: ambiguousMessage(listQuery) };
    return { list: null, listError: `Unknown task list "${listQuery}"` };
}

function ambiguousMessage(query: string): string {
    return `Multiple task lists match "${query}"`;
}

/** Format a parsed quick-capture due value relative to `today` (e.g. `Tomorrow 09:30`). */
export function formatQuickCaptureDue(due: string, hasTime: boolean, today: Date): string {
    const [datePart, timePart] = due.split('T');
    const day = formatRelativeDay(datePart, today);
    if (!hasTime || timePart === undefined) return day;

    const hours = Number(timePart.slice(0, 2));
    const minutes = Number(timePart.slice(3, 5));
    return `${day} ${formatHhMm(hours * 60 + minutes)}`;
}

function formatRelativeDay(datePart: string, today: Date): string {
    const date = parseLocalYmd(datePart);
    if (sameDay(date, today)) return 'Today';
    if (sameDay(date, addDays(today, 1))) return 'Tomorrow';
    return date.toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short' });
}
