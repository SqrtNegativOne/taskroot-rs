import type { AppSettings, AppTaskStatus, TaskPriority } from '../bindings/index.generated';

export type SettingValue = NonNullable<AppSettings[keyof AppSettings]>;

const TASK_STATUSES: readonly string[] = ['todo', 'next-up', 'done', 'cancelled'];

export const DEFAULT_TASK_PRIORITY: TaskPriority = 2;

export function isAppTaskStatus(s: unknown): s is AppTaskStatus {
    return typeof s === 'string' && TASK_STATUSES.includes(s);
}
