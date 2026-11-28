import type { Attachment } from 'svelte/attachments';

export interface ClickOutsideOptions {
    /** Attaches the global listener only while true. Defaults to true. */
    enabled?: boolean;
    /** Event target receiving pointerdown events. Defaults to `document`. */
    target?: Document | Window;
    /** Whether to register in the capture phase. Defaults to false. */
    capture?: boolean;
    /** Optional predicate to ignore specific outside pointerdown events. */
    shouldIgnore?: (event: PointerEvent) => boolean;
}

function attachClickOutside(
    element: HTMLElement,
    onOutside: (event: PointerEvent) => void,
    target: Document | Window,
    capture: boolean,
    shouldIgnore?: (event: PointerEvent) => boolean,
): () => void {
    function handlePointerDown(event: Event) {
        if (!(event instanceof PointerEvent)) return;
        if (shouldIgnore?.(event)) return;
        if (event.target instanceof Node && element.contains(event.target)) return;
        onOutside(event);
    }

    target.addEventListener('pointerdown', handlePointerDown, capture);
    return () => {
        target.removeEventListener('pointerdown', handlePointerDown, capture);
    };
}

/**
 * Svelte 5 attachment that invokes `onOutside` on `pointerdown` outside the
 * attached element. Only registers the global listener while `enabled` is true.
 */
export function clickOutside(
    onOutside: (event: PointerEvent) => void,
    options: ClickOutsideOptions = {},
): Attachment<HTMLElement> {
    const {
        enabled = true,
        target = document,
        capture = false,
        shouldIgnore,
    } = options;

    return (element) => {
        if (!enabled) return;
        return attachClickOutside(element, onOutside, target, capture, shouldIgnore);
    };
}
