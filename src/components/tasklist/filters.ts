import type { AppTask } from '../../lib/domain';

export interface AppFilter {
    column?: string;
    operator?: string;
    value?: unknown;
}

export function computeFilterDefaults(filters: AppFilter[]): Partial<AppTask> {
    const defaults: Partial<AppTask> = {};
    for (const f of filters) {
        if (f.operator === "is" && f.column) {
            const vals = Array.isArray(f.value) ? f.value : [f.value];
            if (vals.length === 1) {
                if (f.column === "status") defaults.status = vals[0] as AppTask["status"];
                if (f.column === "priority") defaults.priority = Number(vals[0]);
                if (f.column === "tag" && typeof vals[0] === 'string') defaults.tags = [vals[0]];
            }
        }
    }
    return defaults;
}

export function checkTaskAgainstFilters(task: AppTask, filters: AppFilter[]): boolean {
    for (const f of filters) {
        if (!f.column || (!f.value && f.value !== 0)) continue;
        let match = false;
        const values = Array.isArray(f.value) ? f.value : [f.value];
        if (values.length === 0) continue;
        
        if (f.column === "status") {
            match = values.includes(task.status ?? "");
        } else if (f.column === "priority") {
            match = values.includes(task.priority ?? 0) || values.includes(String(task.priority));
        } else if (f.column === "tag") {
            match = values.some((v) => (task.tags ?? []).includes(String(v)));
        }
        
        const isKeep = f.operator === "is not" ? !match : match;
        if (!isKeep) return true; // will be filtered out
    }
    return false;
}
