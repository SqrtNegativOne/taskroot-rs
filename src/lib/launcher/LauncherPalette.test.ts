import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke } from '../../test/tauriMock';
import { emptyFilterGroup, filterGroup } from '../domain';
import { ymd } from '../time';
import LauncherPalette from './LauncherPalette.svelte';

vi.mock('@tauri-apps/api/core', async () => {
    const { mockTauriInvoke } = await import('../../test/tauriMock');
    return { invoke: mockTauriInvoke().invoke };
});

const tauri = mockTauriInvoke();

interface PreviewOverrides {
    title?: string;
    due?: string | null;
    hasTime?: boolean;
    tags?: string[];
    priority?: number | null;
    estimateMinutes?: number | null;
    list?: {
        id: string;
        summary: string;
        writable: boolean;
        component: 'VEVENT' | 'VTODO';
    } | null;
    listQuery?: string | null;
    listError?: string | null;
}

function makePreview(overrides: PreviewOverrides = {}) {
    return {
        title: 'Buy milk',
        due: null,
        hasTime: false,
        tags: [],
        priority: null,
        estimateMinutes: null,
        list: null,
        listQuery: null,
        listError: null,
        ...overrides,
    };
}

function callFor(command: string) {
    return tauri.invoke.mock.calls.find(([name]) => name === command);
}

async function type(value: string): Promise<HTMLElement> {
    const input = screen.getByRole('textbox');
    await fireEvent.input(input, { target: { value } });
    return input;
}

describe('LauncherPalette', () => {
    beforeEach(() => {
        tauri.reset();
        tauri.stubCommandValue('query_event_instances', []);
        tauri.stubCommandValue('query_tasks', []);
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it("queries today's agenda while the input is empty", async () => {
        render(LauncherPalette);

        const today = ymd(new Date());
        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'query_event_instances',
                expect.objectContaining({ startDate: today, endDate: today, filters: emptyFilterGroup() }),
            );
        });
        expect(tauri.invoke).toHaveBeenCalledWith(
            'query_tasks',
            { filters: filterGroup([{ column: 'status', operator: 'is not', value: ['done'] }]), sort: [], query: '' },
        );
    });

    it('parses typed text and previews the captured task', async () => {
        tauri.stubCommandValue('parse_quick_capture', makePreview());
        render(LauncherPalette);

        await type('buy milk');

        await waitFor(() => {
            expect(callFor('parse_quick_capture')?.[1]).toEqual({ text: 'buy milk' });
        });
        expect(await screen.findByText('Buy milk')).not.toBeNull();
    });

    it('creates the previewed task with its due, tags, priority, estimate, list and zone', async () => {
        tauri.stubCommandValue(
            'parse_quick_capture',
            makePreview({
                due: '2026-11-20T09:30:00',
                hasTime: true,
                tags: ['work', 'urgent'],
                priority: 2,
                estimateMinutes: 45,
                list: {
                    id: '/tasks/',
                    summary: 'Tasks',
                    writable: true,
                    component: 'VTODO',
                },
                listQuery: '+tasks',
            }),
        );
        render(LauncherPalette);

        const input = await type('buy milk');
        await screen.findByText('Buy milk');
        await fireEvent.keyDown(input, { key: 'Enter' });

        await waitFor(() => expect(callFor('create_task')).toBeDefined());
        expect(callFor('create_task')?.[1]).toMatchObject({
            task: {
                title: 'Buy milk',
                status: 'todo',
                priority: 2,
                due: '2026-11-20T09:30:00',
                est: 45,
                remoteCollectionId: '/tasks/',
                timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
                tags: [{ name: 'work' }, { name: 'urgent' }],
            },
        });
        await waitFor(() => expect(callFor('hide_launcher')).toBeDefined());
    });

    it('does not create a task when the preview reports a list error', async () => {
        tauri.stubCommandValue(
            'parse_quick_capture',
            makePreview({ listError: 'No writable task list' }),
        );
        render(LauncherPalette);

        const input = await type('buy milk');
        await screen.findByText('No writable task list');
        await fireEvent.keyDown(input, { key: 'Enter' });

        await waitFor(() => expect(callFor('parse_quick_capture')).toBeDefined());
        expect(callFor('create_task')).toBeUndefined();
    });

    it('hides the launcher on Escape', async () => {
        render(LauncherPalette);

        await fireEvent.keyDown(screen.getByRole('textbox'), { key: 'Escape' });

        await waitFor(() => expect(callFor('hide_launcher')).toBeDefined());
    });

    it('refreshes the agenda date range after midnight when reopened without remounting', async () => {
        vi.useFakeTimers({ toFake: ['Date'] });
        vi.setSystemTime(new Date(2026, 10, 20, 23, 59, 0));

        render(LauncherPalette);
        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'query_event_instances',
                expect.objectContaining({ startDate: '2026-11-20', endDate: '2026-11-20' }),
            );
        });

        vi.setSystemTime(new Date(2026, 10, 21, 0, 1, 0));
        window.dispatchEvent(new Event('focus'));

        await waitFor(() => {
            expect(tauri.invoke).toHaveBeenCalledWith(
                'query_event_instances',
                expect.objectContaining({ startDate: '2026-11-21', endDate: '2026-11-21' }),
            );
        });
    });
});
