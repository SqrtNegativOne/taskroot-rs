import type { AppTaskStatus } from '../bindings/AppTaskStatus.generated';

export interface AppNote {
    readonly id: string;
    readonly title: string;
    readonly vault: string;
    readonly path: string;
}

export interface AppFilter {
    readonly id?: string;
    readonly column: string;
    readonly operator: string;
    readonly value: unknown;
}

export function isYmdString(s: unknown): s is string {
    return typeof s === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(s);
}

const TASK_STATUSES: readonly string[] = ['todo', 'next-up', 'doing', 'done'];

export function isAppTaskStatus(s: unknown): s is AppTaskStatus {
    return typeof s === 'string' && TASK_STATUSES.includes(s);
}

export type EventType = 'busy' | 'info' | 'log';

export function toEventType(raw: string | undefined, fallback: EventType): EventType {
    if (raw === 'info' || raw === 'busy' || raw === 'log') {
        return raw;
    }
    return fallback;
}

export type OptionalKeysOf<T> = {
    [K in keyof T]-?: undefined extends T[K] ? K : never;
}[keyof T];

export interface EditingSession<T> {
    set<K extends keyof T>(key: K, value: T[K]): EditingSession<T>;
    clear(key: OptionalKeysOf<T>): EditingSession<T>;
    done(): T;
}

export function editing<T extends { readonly id: string }>(item: T): EditingSession<T> {
    return {
        set<K extends keyof T>(key: K, value: T[K]): EditingSession<T> {
            return editing<T>({ ...item, [key]: value });
        },
        clear(key: OptionalKeysOf<T>): EditingSession<T> {
            const copy = { ...item };
            delete (copy as Record<string, unknown>)[key as string];
            return editing<T>(copy);
        },
        done: () => item,
    };
}
