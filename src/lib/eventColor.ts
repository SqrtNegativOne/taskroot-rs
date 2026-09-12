const HEX_RGB_PATTERN = /^#[0-9a-fA-F]{6}$/;
const EVENT_BACKGROUND_ALPHA = '40';

/**
 * Append an 8-bit alpha suffix to a 6-digit hex color. Colors that are already
 * in another notation (or carry their own alpha) are returned unchanged.
 */
export function withAlpha(color: string, alphaHex: string = EVENT_BACKGROUND_ALPHA): string {
    return HEX_RGB_PATTERN.test(color) ? `${color}${alphaHex}` : color;
}

/**
 * Inline CSS custom properties shared by every event render surface so the
 * date grid and day timeline cannot drift apart. Empty when the event has no
 * color, letting each surface fall back to its own default.
 */
export function eventColorVars(color?: string): string {
    if (!color) return '';
    return `--ev-color: ${color}; --ev-bg: ${withAlpha(color)};`;
}
