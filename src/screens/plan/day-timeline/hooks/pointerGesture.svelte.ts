import { SvelteSet } from 'svelte/reactivity';

export interface PointerGestureHandlers {
    onMove?: (e: PointerEvent) => void;
    onEnd?: (e: PointerEvent) => void;
    onCancel?: () => void;
}

export type PointerGestureTeardown = () => void;

export type PointerGestureRecognizer = (
    handlers: PointerGestureHandlers,
) => PointerGestureTeardown;

export function createPointerGestureRecognizer(): PointerGestureRecognizer {
    const activeTeardowns = new SvelteSet<PointerGestureTeardown>();

    $effect(() => {
        return () => {
            for (const teardown of activeTeardowns) teardown();
        };
    });

    return (handlers) => {
        const onMove = (e: PointerEvent) => handlers.onMove?.(e);

        const detach = () => {
            window.removeEventListener('pointermove', onMove);
            window.removeEventListener('pointerup', finish);
            window.removeEventListener('pointercancel', cancel);
            activeTeardowns.delete(detach);
        };

        const finish = (e: PointerEvent) => {
            detach();
            handlers.onEnd?.(e);
        };

        const cancel = () => {
            detach();
            handlers.onCancel?.();
        };

        window.addEventListener('pointermove', onMove);
        window.addEventListener('pointerup', finish);
        window.addEventListener('pointercancel', cancel);
        activeTeardowns.add(detach);
        return detach;
    };
}
