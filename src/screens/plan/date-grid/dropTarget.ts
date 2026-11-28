import { MINUTES_PER_DAY, snappedDropMinuteFrom } from '../snapping';
import type { DragStateTarget } from '../day-timeline/types';

/** Translate the element under the pointer into a drop target. */
export function targetFromElement(
    element: Element,
    clientY: number,
    duration: number,
): DragStateTarget | undefined {
    const kind = element.getAttribute('data-drop-kind');
    const date = element.getAttribute('data-drop-date');
    if (!date) return undefined;
    if (kind === 'grid-day') return { kind, date };
    if (kind !== 'day-time') return undefined;

    const rawStart = Number(element.getAttribute('data-start-minute') ?? 0);
    const startMinute = Number.isFinite(rawStart)
        ? Math.min(MINUTES_PER_DAY, Math.max(0, rawStart))
        : 0;
    const rect = element.getBoundingClientRect();
    const minute = snappedDropMinuteFrom(clientY - rect.top, duration, startMinute);
    return { kind, date, minute, duration };
}

/** The drop target under a client point, if any. */
export function dropTargetAt(
    clientX: number,
    clientY: number,
    duration: number,
): DragStateTarget | undefined {
    const element = document.elementFromPoint(clientX, clientY)?.closest('[data-drop-kind]');
    return element ? targetFromElement(element, clientY, duration) : undefined;
}

/** Swallow the click that follows a completed drag, so it does not open the inspector. */
export function suppressUpcomingClick(): void {
    const suppress = (click: Event) => {
        click.stopPropagation();
        click.preventDefault();
    };
    window.addEventListener('click', suppress, true);
    setTimeout(() => {
        window.removeEventListener('click', suppress, true);
    }, 0);
}
