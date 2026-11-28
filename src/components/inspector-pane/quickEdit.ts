import type { AppCalendar, AppTask, TaskPatch, TaskPriority } from '../../lib/domain';
import { formatQuickCaptureDue, parseQuickCapture } from '../../lib/launcher/quickCapture';
import { parseLocalYmd, ymd } from '../../lib/time';

export interface TaskTitleQuickEdit {
    /** Cleaned title with recognized tokens stripped (preserves `currentTitle` if only properties were typed). */
    title: string;
    due: string | null;
    hasTime: boolean;
    tags: string[];
    priority: TaskPriority | null;
    estimateMinutes: number | null;
    /** Resolved target list only when `+list` was explicitly typed. */
    list: AppCalendar | null;
    listQuery: string | null;
    listError: string | null;
    /** True when at least one valid property sigil or natural-language date was recognized. */
    hasProperties: boolean;
    /** Human-readable summary items for the live inspector preview. */
    previewParts: string[];
}

const TRAILING_INCOMPLETE_SIGIL = /(?:^|\s)[#+^!]$/;

/**
 * Parse a task title edit using the same sigils and natural-language date
 * grammar as quick capture, scoped to modifying an existing task.
 */
export function parseTaskTitleQuickEdit(
    input: string,
    currentTitle: string,
    referenceDate: Date,
    calendars: readonly AppCalendar[],
): TaskTitleQuickEdit {
    const preview = parseQuickCapture(input, referenceDate, calendars);
    const tags = dedupeStrings(preview.tags);
    const list = preview.listQuery !== null ? preview.list : null;
    const listError = preview.listQuery !== null ? preview.listError : null;
    const hasProperties =
        preview.due !== null ||
        tags.length > 0 ||
        preview.priority !== null ||
        preview.estimateMinutes !== null ||
        list !== null;

    const title =
        preview.title.trim() !== ''
            ? preview.title
            : hasProperties
                ? currentTitle
                : input;

    const previewParts = formatPreviewParts(
        {
            due: preview.due,
            hasTime: preview.hasTime,
            tags,
            priority: preview.priority,
            estimateMinutes: preview.estimateMinutes,
            list,
        },
        referenceDate,
    );

    return {
        title,
        due: preview.due,
        hasTime: preview.hasTime,
        tags,
        priority: preview.priority,
        estimateMinutes: preview.estimateMinutes,
        list,
        listQuery: preview.listQuery,
        listError,
        hasProperties,
        previewParts,
    };
}

/** True when the input ends with a bare sigil prefix (`#`, `+`, `^`, `!`) still being typed. */
export function hasTrailingIncompleteSigil(input: string): boolean {
    return TRAILING_INCOMPLETE_SIGIL.test(input);
}

/**
 * Build the minimal field patches to apply a parsed title quick-edit onto an
 * existing task. Tags are merged onto existing tags without duplicates.
 */
export function buildTaskQuickEditPatches(
    task: AppTask,
    edit: TaskTitleQuickEdit,
    timezone = Intl.DateTimeFormat().resolvedOptions().timeZone,
): TaskPatch[] {
    const patches: TaskPatch[] = [];

    if (edit.title !== task.title) {
        patches.push({ field: 'title', value: edit.title });
    }
    if (edit.due !== null) {
        patches.push({ field: 'due', value: edit.due });
        if (edit.hasTime) {
            patches.push({ field: 'timezone', value: timezone });
        }
    }
    if (edit.priority !== null) {
        patches.push({ field: 'priority', value: edit.priority });
    }
    if (edit.estimateMinutes !== null) {
        patches.push({ field: 'est', value: edit.estimateMinutes || null });
    }
    if (edit.tags.length > 0) {
        const existing = task.tags ?? [];
        const existingNames = new Set(existing.map((tag) => tag.name));
        const added = edit.tags.filter((name) => !existingNames.has(name));
        if (added.length > 0) {
            patches.push({
                field: 'tags',
                value: [
                    ...existing,
                    ...added.map((name) => ({ id: crypto.randomUUID(), name })),
                ],
            });
        }
    }
    if (edit.listQuery !== null && edit.list !== null && edit.list.id !== task.remoteCollectionId) {
        patches.push({ field: 'remoteCollectionId', value: edit.list.id });
    }

    return patches;
}

function dedupeStrings(items: readonly string[]): string[] {
    const seen = new Set<string>();
    const result: string[] = [];
    for (const item of items) {
        if (seen.has(item)) continue;
        seen.add(item);
        result.push(item);
    }
    return result;
}

function formatPreviewParts(
    edit: Pick<TaskTitleQuickEdit, 'due' | 'hasTime' | 'tags' | 'priority' | 'estimateMinutes' | 'list'>,
    referenceDate: Date,
): string[] {
    const parts: string[] = [];
    if (edit.due !== null) {
        const today = parseLocalYmd(ymd(referenceDate));
        parts.push(`Due ${formatQuickCaptureDue(edit.due, edit.hasTime, today)}`);
    }
    if (edit.tags.length > 0) {
        parts.push(edit.tags.map((tag) => `#${tag}`).join(' '));
    }
    if (edit.priority !== null) {
        parts.push(`P${edit.priority}`);
    }
    if (edit.estimateMinutes !== null) {
        parts.push(edit.estimateMinutes === 0 ? 'Clear estimate' : `${edit.estimateMinutes} min`);
    }
    if (edit.list !== null) {
        parts.push(edit.list.summary);
    }
    return parts;
}
