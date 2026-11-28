import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { toast } from 'svelte-sonner';
import { mockTauriInvoke } from '../../test/tauriMock';
import type { AppEvent, AppTask } from '../../lib/domain';
import TagsScreen from './TagsScreen.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

vi.mock('svelte-sonner', () => ({
    toast: {
        error: vi.fn(),
        success: vi.fn(),
        info: vi.fn(),
        custom: vi.fn(),
    },
    Toaster: vi.fn(),
}));

const tauri = mockTauriInvoke();

const sampleTask: AppTask = {
    id: 'task-1',
    title: 'Ship release',
    status: 'todo',
    tags: [{ id: 'work', name: 'work' }],
};

const sampleEvent: AppEvent = {
    id: 'event-1',
    title: 'Planning sync',
    startTime: '2026-09-07T09:00:00',
    endTime: '2026-09-07T10:00:00',
    tags: [{ id: 'urgent', name: 'urgent' }],
};

function renameCalls() {
    return tauri.invoke.mock.calls.filter(([name]) => name === 'rename_tag');
}

describe('TagsScreen tag renaming', () => {
    beforeEach(() => {
        tauri.reset();
        vi.clearAllMocks();
        tauri.stubCommandValue('query_tasks', [sampleTask]);
        tauri.stubCommandValue('query_events', [sampleEvent]);
    });

    afterEach(() => {
        vi.restoreAllMocks();
    });

    it('opens an inline editor prefilled with the tag name when Rename is clicked', async () => {
        render(TagsScreen);

        const renameButton = await screen.findByRole('button', { name: 'Rename tag work' });
        await fireEvent.click(renameButton);

        const input = screen.getByRole('textbox', { name: 'Tag name' }) as HTMLInputElement;
        expect(input.value).toBe('work');
        expect(document.activeElement).toBe(input);
        expect(screen.queryByRole('heading', { name: 'work' })).toBeNull();
    });

    it('submits a trimmed new name via rename_tag and exits edit mode on success', async () => {
        render(TagsScreen);

        await fireEvent.click(await screen.findByRole('button', { name: 'Rename tag work' }));
        const input = screen.getByRole('textbox', { name: 'Tag name' });
        await fireEvent.input(input, { target: { value: '  office  ' } });
        await fireEvent.click(screen.getByRole('button', { name: 'Save tag name' }));

        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'rename_tag',
                { from: 'work', to: 'office' },
                undefined,
            );
        });
        await waitFor(() => {
            expect(screen.queryByRole('textbox', { name: 'Tag name' })).toBeNull();
        });
    });

    it('submits on Enter and disables controls while the request is pending', async () => {
        let resolveRename: () => void = () => {};
        tauri.stubCommand(
            'rename_tag',
            () => new Promise<void>((resolve) => {
                resolveRename = resolve;
            }),
        );

        render(TagsScreen);

        await fireEvent.click(await screen.findByRole('button', { name: 'Rename tag urgent' }));
        const input = screen.getByRole('textbox', { name: 'Tag name' }) as HTMLInputElement;
        await fireEvent.input(input, { target: { value: 'critical' } });
        await fireEvent.keyDown(input, { key: 'Enter' });

        await waitFor(() => {
            expect(input.disabled).toBe(true);
        });
        expect((screen.getByRole('button', { name: 'Save tag name' }) as HTMLButtonElement).disabled).toBe(true);
        expect((screen.getByRole('button', { name: 'Cancel rename' }) as HTMLButtonElement).disabled).toBe(true);

        resolveRename();

        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'rename_tag',
                { from: 'urgent', to: 'critical' },
                undefined,
            );
            expect(screen.queryByRole('textbox', { name: 'Tag name' })).toBeNull();
        });
    });

    it('closes edit mode without calling rename_tag on Escape, Cancel, or unchanged name', async () => {
        render(TagsScreen);

        await fireEvent.click(await screen.findByRole('button', { name: 'Rename tag work' }));
        const input = screen.getByRole('textbox', { name: 'Tag name' });
        await fireEvent.input(input, { target: { value: 'other' } });
        await fireEvent.keyDown(input, { key: 'Escape' });

        expect(screen.queryByRole('textbox', { name: 'Tag name' })).toBeNull();
        expect(renameCalls()).toHaveLength(0);

        await fireEvent.click(screen.getByRole('button', { name: 'Rename tag work' }));
        const secondInput = screen.getByRole('textbox', { name: 'Tag name' });
        await fireEvent.input(secondInput, { target: { value: '  work  ' } });
        await fireEvent.keyDown(secondInput, { key: 'Enter' });

        expect(screen.queryByRole('textbox', { name: 'Tag name' })).toBeNull();
        expect(renameCalls()).toHaveLength(0);

        await fireEvent.click(screen.getByRole('button', { name: 'Rename tag work' }));
        await fireEvent.click(screen.getByRole('button', { name: 'Cancel rename' }));

        expect(screen.queryByRole('textbox', { name: 'Tag name' })).toBeNull();
        expect(renameCalls()).toHaveLength(0);
    });

    it('rejects empty and comma-containing names without calling rename_tag', async () => {
        render(TagsScreen);

        await fireEvent.click(await screen.findByRole('button', { name: 'Rename tag work' }));
        const input = screen.getByRole('textbox', { name: 'Tag name' });
        const saveBtn = screen.getByRole('button', { name: 'Save tag name' }) as HTMLButtonElement;

        await fireEvent.input(input, { target: { value: '   ' } });
        expect(saveBtn.disabled).toBe(true);
        await fireEvent.keyDown(input, { key: 'Enter' });
        expect(screen.getByRole('textbox', { name: 'Tag name' })).toBeTruthy();
        expect(renameCalls()).toHaveLength(0);

        await fireEvent.input(input, { target: { value: 'work,home' } });
        expect(saveBtn.disabled).toBe(true);
        await fireEvent.keyDown(input, { key: 'Enter' });
        expect(screen.getByRole('textbox', { name: 'Tag name' })).toBeTruthy();
        expect(renameCalls()).toHaveLength(0);
    });

    it('keeps edit mode open and surfaces a toast if rename_tag fails', async () => {
        tauri.stubCommand('rename_tag', () => Promise.reject({ code: 'remote', message: 'server down' }));
        render(TagsScreen);

        await fireEvent.click(await screen.findByRole('button', { name: 'Rename tag work' }));
        const input = screen.getByRole('textbox', { name: 'Tag name' }) as HTMLInputElement;
        await fireEvent.input(input, { target: { value: 'office' } });
        await fireEvent.click(screen.getByRole('button', { name: 'Save tag name' }));

        await waitFor(() => {
            expect(toast.custom).toHaveBeenCalledWith(
                expect.anything(),
                expect.objectContaining({
                    componentProps: { message: 'Failed to rename tag: server down' },
                }),
            );
        });
        expect((screen.getByRole('textbox', { name: 'Tag name' }) as HTMLInputElement).value).toBe('office');
    });

    it('renders wall-clock dates and times for UTC timestamps instead of slicing raw UTC strings', async () => {
        tauri.stubCommandValue('query_tasks', [
            {
                id: 'utc-task',
                title: 'Timezone task',
                status: 'todo',
                due: '2026-09-07T23:30:00Z',
                timezone: 'Asia/Kolkata',
                tags: [{ id: 'tz', name: 'tz' }],
            },
        ]);
        tauri.stubCommandValue('query_events', [
            {
                id: 'utc-event',
                title: 'Timezone event',
                startTime: '2026-09-07T23:30:00Z',
                endTime: '2026-09-08T00:30:00Z',
                isAllDay: false,
                timezone: 'Asia/Kolkata',
                tags: [{ id: 'tz', name: 'tz' }],
            },
        ]);

        render(TagsScreen);

        expect(await screen.findByText('2026-09-08 05:00')).toBeTruthy();
        expect(screen.getByText('todo · due 2026-09-08')).toBeTruthy();
    });
});
