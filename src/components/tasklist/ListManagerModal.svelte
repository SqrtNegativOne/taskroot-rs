<script lang="ts">
    import { Dialog } from 'bits-ui';
    import type { AppCalendar } from '../../lib/domain';
    import type { Result } from 'neverthrow';
    import type { AppError } from '../../lib/safeInvoke.svelte';
    import { store } from '../../lib/store.svelte';

    interface Props {
        isOpen: boolean;
        calendars: AppCalendar[];
        onClose: () => void;
    }

    let { isOpen, calendars, onClose }: Props = $props();

    let newName = $state('');
    let newComponent = $state<'VTODO' | 'VEVENT'>('VTODO');
    let busy = $state(false);

    async function run(
        operation: string,
        action: () => Promise<Result<void, AppError>>,
    ): Promise<void> {
        busy = true;
        await store.mutate(operation, action);
        busy = false;
    }

    async function handleCreate(): Promise<void> {
        const name = newName.trim();
        if (!name) return;
        await run('Failed to create list', () => store.createCalendar(name, newComponent));
        newName = '';
    }

    async function handleRename(calendar: AppCalendar, value: string): Promise<void> {
        const name = value.trim();
        if (!name || name === calendar.summary) return;
        await run('Failed to rename list', () => store.renameCalendar(calendar.id, name));
    }

    async function handleColor(calendar: AppCalendar, value: string): Promise<void> {
        await run('Failed to recolour list', () => store.setCalendarColor(calendar.id, value));
    }

    async function handleClearColor(calendar: AppCalendar): Promise<void> {
        await run('Failed to clear colour', () => store.setCalendarColor(calendar.id, null));
    }

    async function handleDelete(calendar: AppCalendar): Promise<void> {
        const kind = calendar.component === 'VTODO' ? 'tasks' : 'events';
        if (!confirm(`Delete "${calendar.summary}" and every ${kind} in it? This cannot be undone.`)) {
            return;
        }
        await run('Failed to delete list', () => store.deleteCalendar(calendar.id));
    }

    function handleOpenChange(open: boolean): void {
        if (!open) onClose();
    }
</script>

<Dialog.Root open={isOpen} onOpenChange={handleOpenChange}>
    <Dialog.Portal>
        <Dialog.Overlay
            style="position: fixed; inset: 0; background: rgba(0,0,0,0.5); z-index: 99999;"
        />
        <Dialog.Content
            style="position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%); background: var(--bg-panel, #1e1e1e); padding: 20px 24px; border-radius: 8px; width: 460px; max-height: 80vh; display: flex; flex-direction: column; box-shadow: 0 4px 12px rgba(0,0,0,0.3); color: var(--fg); z-index: 100000;"
        >
            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 16px;">
                <Dialog.Title level={3} style="margin: 0; font-size: 16px; font-weight: 600;">Lists</Dialog.Title>
                <button class="icon-btn" onclick={onClose} aria-label="Close">✕</button>
            </div>

            <div style="overflow-y: auto; display: flex; flex-direction: column; gap: 6px; margin-bottom: 16px;">
                {#each calendars as calendar (calendar.id)}
                    <div style="display: flex; align-items: center; gap: 8px; padding: 4px 0;">
                        <input
                            type="color"
                            value={calendar.color?.slice(0, 7) ?? '#888888'}
                            disabled={!calendar.writable || busy}
                            onchange={(e) => handleColor(calendar, e.currentTarget.value)}
                            style="width: 22px; height: 22px; padding: 0; border: none; background: transparent; cursor: pointer;"
                            aria-label={`Colour of ${calendar.summary}`}
                        />
                        <input
                            type="text"
                            value={calendar.summary}
                            disabled={!calendar.writable || busy}
                            onchange={(e) => handleRename(calendar, e.currentTarget.value)}
                            style="flex: 1; min-width: 0; background: transparent; border: 1px solid transparent; border-radius: 4px; color: var(--fg); padding: 4px 6px; font: inherit;"
                            onfocus={(e) => { e.currentTarget.style.borderColor = 'var(--border)'; }}
                            onblur={(e) => { e.currentTarget.style.borderColor = 'transparent'; }}
                        />
                        <span style="font-size: 11px; color: var(--fg-dim);">
                            {calendar.component === 'VTODO' ? 'tasks' : 'calendar'}
                            {#if !calendar.writable} · read-only{/if}
                        </span>
                        {#if calendar.color}
                            <button
                                class="icon-btn"
                                title="Clear colour"
                                disabled={!calendar.writable || busy}
                                onclick={() => handleClearColor(calendar)}
                            >⨯</button>
                        {/if}
                        <button
                            class="icon-btn"
                            title="Delete list"
                            disabled={!calendar.writable || busy}
                            onclick={() => handleDelete(calendar)}
                        >🗑</button>
                    </div>
                {/each}
                {#if calendars.length === 0}
                    <p style="color: var(--fg-dim); font-size: 13px; margin: 0;">No lists on the server yet.</p>
                {/if}
            </div>

            <div style="display: flex; gap: 8px; border-top: 1px solid var(--border); padding-top: 16px;">
                <input
                    type="text"
                    bind:value={newName}
                    placeholder="New list name"
                    style="flex: 1; min-width: 0; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 4px; color: var(--fg); padding: 6px 8px; font: inherit;"
                    onkeydown={(e) => { if (e.key === 'Enter') void handleCreate(); }}
                />
                <select
                    bind:value={newComponent}
                    style="background: var(--bg-surface); border: 1px solid var(--border); border-radius: 4px; color: var(--fg); padding: 6px 8px;"
                >
                    <option value="VTODO">Tasks</option>
                    <option value="VEVENT">Calendar</option>
                </select>
                <button
                    disabled={busy || !newName.trim()}
                    onclick={() => void handleCreate()}
                    style="background: var(--accent, #3b82f6); border: none; color: #fff; border-radius: 4px; padding: 6px 14px; cursor: pointer;"
                >Add</button>
            </div>
        </Dialog.Content>
    </Dialog.Portal>
</Dialog.Root>
