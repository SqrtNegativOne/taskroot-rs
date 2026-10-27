import type { AppTaskStatus } from '../bindings/AppTaskStatus.generated';

const TASK_STATUSES: readonly string[] = ['todo', 'next-up', 'doing', 'done'];

export function isAppTaskStatus(s: unknown): s is AppTaskStatus {
    return typeof s === 'string' && TASK_STATUSES.includes(s);
}
