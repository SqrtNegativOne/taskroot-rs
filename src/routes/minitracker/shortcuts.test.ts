import { describe, expect, it } from 'vitest';
import {
    resolveMinitrackerKeyCommand,
    ShiftChordTracker,
    type KeyDispatchContext,
    type KeyEventLike,
} from './shortcuts';

function keyEvent(overrides: Partial<KeyEventLike> = {}): KeyEventLike {
    return {
        key: '',
        code: '',
        ctrlKey: false,
        altKey: false,
        metaKey: false,
        shiftKey: false,
        ...overrides,
    };
}

const DEFAULT_CTX: KeyDispatchContext = {
    isTyping: false,
    menuOpen: false,
    isPaused: false,
};

describe('ShiftChordTracker & resolveMinitrackerKeyCommand', () => {
    it('triggers toggleBreak once when both ShiftLeft and ShiftRight are held, then rearms after release', () => {
        const chord = new ShiftChordTracker();

        expect(resolveMinitrackerKeyCommand(keyEvent({ code: 'ShiftLeft', key: 'Shift' }), DEFAULT_CTX, chord)).toBeNull();
        expect(resolveMinitrackerKeyCommand(keyEvent({ code: 'ShiftRight', key: 'Shift' }), DEFAULT_CTX, chord)).toEqual({
            kind: 'toggleBreak',
        });
        // Repeated keydown while held does not re-fire
        expect(resolveMinitrackerKeyCommand(keyEvent({ code: 'ShiftRight', key: 'Shift' }), DEFAULT_CTX, chord)).toBeNull();

        chord.onKeyUp('ShiftRight');
        expect(resolveMinitrackerKeyCommand(keyEvent({ code: 'ShiftRight', key: 'Shift' }), DEFAULT_CTX, chord)).toEqual({
            kind: 'toggleBreak',
        });

        // Blur reset clears held state
        chord.reset();
        expect(resolveMinitrackerKeyCommand(keyEvent({ code: 'ShiftRight', key: 'Shift' }), DEFAULT_CTX, chord)).toBeNull();
    });

    it('fires shift-chord even when typing, but otherwise allows only Escape to close an open menu while typing', () => {
        const chord = new ShiftChordTracker();
        const typingOpen: KeyDispatchContext = { isTyping: true, menuOpen: true, isPaused: true };

        expect(resolveMinitrackerKeyCommand(keyEvent({ key: 'h' }), typingOpen, chord)).toBeNull();
        expect(resolveMinitrackerKeyCommand(keyEvent({ code: 'Space', key: ' ' }), typingOpen, chord)).toBeNull();
        expect(resolveMinitrackerKeyCommand(keyEvent({ key: 'p' }), typingOpen, chord)).toBeNull();
        expect(resolveMinitrackerKeyCommand(keyEvent({ key: 'Escape' }), typingOpen, chord)).toEqual({
            kind: 'closeMenu',
        });
        expect(
            resolveMinitrackerKeyCommand(keyEvent({ key: 'Escape' }), { ...typingOpen, menuOpen: false }, chord),
        ).toBeNull();

        expect(resolveMinitrackerKeyCommand(keyEvent({ code: 'ShiftLeft' }), typingOpen, chord)).toBeNull();
        expect(resolveMinitrackerKeyCommand(keyEvent({ code: 'ShiftRight' }), typingOpen, chord)).toEqual({
            kind: 'toggleBreak',
        });
    });

    it('dispatches dim, restore, pause, pause-adjustment, and space transport shortcuts', () => {
        const chord = new ShiftChordTracker();

        expect(resolveMinitrackerKeyCommand(keyEvent({ key: 'h' }), DEFAULT_CTX, chord)).toEqual({
            kind: 'toggleDimmed',
        });
        expect(resolveMinitrackerKeyCommand(keyEvent({ key: 'h', ctrlKey: true }), DEFAULT_CTX, chord)).toBeNull();
        expect(
            resolveMinitrackerKeyCommand(keyEvent({ key: 'r', ctrlKey: true, altKey: true }), DEFAULT_CTX, chord),
        ).toEqual({ kind: 'restoreMainWindow' });
        expect(resolveMinitrackerKeyCommand(keyEvent({ key: 'r' }), DEFAULT_CTX, chord)).toBeNull();

        expect(resolveMinitrackerKeyCommand(keyEvent({ key: 'p' }), DEFAULT_CTX, chord)).toEqual({
            kind: 'togglePause',
        });
        expect(resolveMinitrackerKeyCommand(keyEvent({ key: 'P' }), DEFAULT_CTX, chord)).toEqual({
            kind: 'togglePause',
        });
        expect(resolveMinitrackerKeyCommand(keyEvent({ code: 'Space', key: ' ' }), DEFAULT_CTX, chord)).toEqual({
            kind: 'toggleTransport',
        });
    });

    it('only adjusts pause duration on arrow keys while paused', () => {
        const chord = new ShiftChordTracker();
        const pausedCtx: KeyDispatchContext = { isTyping: false, menuOpen: false, isPaused: true };

        expect(resolveMinitrackerKeyCommand(keyEvent({ key: 'ArrowUp' }), DEFAULT_CTX, chord)).toBeNull();
        expect(resolveMinitrackerKeyCommand(keyEvent({ key: 'ArrowDown' }), DEFAULT_CTX, chord)).toBeNull();

        expect(resolveMinitrackerKeyCommand(keyEvent({ key: 'ArrowUp' }), pausedCtx, chord)).toEqual({
            kind: 'adjustPause',
            delta: 1,
        });
        expect(resolveMinitrackerKeyCommand(keyEvent({ key: 'ArrowRight' }), pausedCtx, chord)).toEqual({
            kind: 'adjustPause',
            delta: 1,
        });
        expect(resolveMinitrackerKeyCommand(keyEvent({ key: 'ArrowDown' }), pausedCtx, chord)).toEqual({
            kind: 'adjustPause',
            delta: -1,
        });
        expect(resolveMinitrackerKeyCommand(keyEvent({ key: 'ArrowLeft' }), pausedCtx, chord)).toEqual({
            kind: 'adjustPause',
            delta: -1,
        });
    });
});
