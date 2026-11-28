export type ShortcutAction = 'toggleDimmed' | 'restoreMainWindow';

export interface KeyboardShortcut {
    readonly key: string;
    readonly ctrl?: boolean;
    readonly alt?: boolean;
    readonly action: ShortcutAction;
}

export interface KeyEventLike {
    readonly key: string;
    readonly code: string;
    readonly ctrlKey: boolean;
    readonly altKey: boolean;
    readonly metaKey: boolean;
    readonly shiftKey: boolean;
}

export type MinitrackerKeyCommand =
    | { readonly kind: 'toggleBreak' }
    | { readonly kind: 'closeMenu' }
    | { readonly kind: 'toggleDimmed' }
    | { readonly kind: 'restoreMainWindow' }
    | { readonly kind: 'togglePause' }
    | { readonly kind: 'adjustPause'; readonly delta: 1 | -1 }
    | { readonly kind: 'toggleTransport' };

export const KEYBOARD_SHORTCUTS: readonly KeyboardShortcut[] = [
    { key: 'h', action: 'toggleDimmed' },
    { key: 'r', ctrl: true, alt: true, action: 'restoreMainWindow' },
];

export function matchesShortcut(e: KeyEventLike, shortcut: KeyboardShortcut): boolean {
    return (
        e.key === shortcut.key &&
        e.ctrlKey === Boolean(shortcut.ctrl) &&
        e.altKey === Boolean(shortcut.alt) &&
        !e.metaKey &&
        !e.shiftKey
    );
}

export function isTypingTarget(target: EventTarget | null): boolean {
    return (
        typeof HTMLElement !== 'undefined' &&
        target instanceof HTMLElement &&
        target.matches('input, textarea, [contenteditable]')
    );
}

export class ShiftChordTracker {
    private leftDown = false;
    private rightDown = false;
    private fired = false;

    /** Returns `'chord'` when both shifts are held for the first time, `'shift'` when a shift key was tracked, or `'none'`. */
    onKeyDown(code: string): 'chord' | 'shift' | 'none' {
        if (code === 'ShiftLeft') {
            this.leftDown = true;
        } else if (code === 'ShiftRight') {
            this.rightDown = true;
        } else {
            return 'none';
        }

        if (this.leftDown && this.rightDown && !this.fired) {
            this.fired = true;
            return 'chord';
        }
        return 'shift';
    }

    onKeyUp(code: string): void {
        if (code === 'ShiftLeft') {
            this.leftDown = false;
        } else if (code === 'ShiftRight') {
            this.rightDown = false;
        } else {
            return;
        }
        this.fired = false;
    }

    reset(): void {
        this.leftDown = false;
        this.rightDown = false;
        this.fired = false;
    }
}

export interface KeyDispatchContext {
    readonly isTyping: boolean;
    readonly menuOpen: boolean;
    readonly isPaused: boolean;
}

function resolvePauseCommand(key: string, isPaused: boolean): MinitrackerKeyCommand | null {
    if (key === 'p' || key === 'P') {
        return { kind: 'togglePause' };
    }
    if (!isPaused) return null;
    if (key === 'ArrowUp' || key === 'ArrowRight') {
        return { kind: 'adjustPause', delta: 1 };
    }
    if (key === 'ArrowDown' || key === 'ArrowLeft') {
        return { kind: 'adjustPause', delta: -1 };
    }
    return null;
}

export function resolveMinitrackerKeyCommand(
    e: KeyEventLike,
    context: KeyDispatchContext,
    shiftChord: ShiftChordTracker,
): MinitrackerKeyCommand | null {
    const shiftResult = shiftChord.onKeyDown(e.code);
    if (shiftResult === 'chord') return { kind: 'toggleBreak' };
    if (shiftResult === 'shift') return null;

    if (context.isTyping) {
        return e.key === 'Escape' && context.menuOpen ? { kind: 'closeMenu' } : null;
    }

    const shortcut = KEYBOARD_SHORTCUTS.find((candidate) => matchesShortcut(e, candidate));
    if (shortcut !== undefined) {
        return { kind: shortcut.action };
    }

    if (e.key === 'Escape' && context.menuOpen) {
        return { kind: 'closeMenu' };
    }

    const pauseCmd = resolvePauseCommand(e.key, context.isPaused);
    if (pauseCmd !== null) return pauseCmd;

    if (e.code === 'Space') {
        return { kind: 'toggleTransport' };
    }

    return null;
}
