import { untrack } from 'svelte';
import { eventColorCss, parseColor, type EventTheme, type Rgb } from '../eventColor';

const FALLBACK_THEME: EventTheme = {
    surface: [34, 43, 56],
    onSurface: [227, 234, 244],
    muted: [169, 184, 204],
    style: 'tint',
};

let revision = $state(0);
let canvasContext: CanvasRenderingContext2D | null | undefined;

function readRevision(): number {
    return revision;
}

function canvas(): CanvasRenderingContext2D | undefined {
    if (canvasContext !== undefined) return canvasContext ?? undefined;
    if (
        typeof document === 'undefined'
        || typeof navigator === 'undefined'
        || /jsdom/i.test(navigator.userAgent)
    ) {
        canvasContext = null;
        return undefined;
    }
    canvasContext = document.createElement('canvas').getContext('2d');
    return canvasContext ?? undefined;
}

function resolveCssColor(value: string, fallback: Rgb): Rgb {
    const raw = value.trim();
    if (!raw) return fallback;
    const parsed = parseColor(raw);
    if (parsed) return parsed;

    const context = canvas();
    if (!context) return fallback;
    context.fillStyle = raw;
    context.fillRect(0, 0, 1, 1);
    const [red, green, blue] = context.getImageData(0, 0, 1, 1).data;
    return [red, green, blue];
}

function readTheme(): EventTheme {
    if (typeof document === 'undefined') return FALLBACK_THEME;
    const styles = getComputedStyle(document.documentElement);
    const style = styles.getPropertyValue('--event-style').trim();
    return {
        surface: resolveCssColor(styles.getPropertyValue('--bg-surface'), FALLBACK_THEME.surface),
        onSurface: resolveCssColor(styles.getPropertyValue('--fg'), FALLBACK_THEME.onSurface),
        muted: resolveCssColor(styles.getPropertyValue('--fg-muted'), FALLBACK_THEME.muted),
        style: style === 'solid' ? 'solid' : 'tint',
    };
}

/** Re-renders every event colour after a palette switch. */
export function notifyEventThemeChanged(): void {
    untrack(() => { revision += 1; });
}

/**
 * Builds the event colour variables from the active palette. Derived colours
 * are never cached, so a theme switch always recomputes them.
 */
export function eventColorVars(color?: string): string {
    readRevision();
    return eventColorCss(color, readTheme());
}
