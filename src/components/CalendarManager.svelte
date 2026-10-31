<script lang="ts">
    import type { Result } from 'neverthrow';
    import type { AppCalendar } from '../lib/domain';
    import type { AppError } from '../lib/safeInvoke.svelte';
    import { store } from '../lib/store.svelte';

    interface Props {
        calendars: AppCalendar[];
    }

    let { calendars }: Props = $props();

    let newName = $state('');
    let newComponent = $state<'VTODO' | 'VEVENT'>('VTODO');
    let busy = $state(false);

    function kindLabel(calendar: AppCalendar): string {
        return calendar.component === 'VTODO' ? 'Tasks' : 'Events';
    }

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
        const question = `Delete "${calendar.summary}" and every ${kind} in it? This cannot be undone.`;
        if (!confirm(question)) return;
        await run('Failed to delete list', () => store.deleteCalendar(calendar.id));
    }
</script>

<ul class="calendars">
    {#each calendars as calendar (calendar.id)}
        <li class="row">
            <input
                type="color"
                class="colour"
                value={calendar.color?.slice(0, 7) ?? '#888888'}
                disabled={!calendar.writable || busy}
                onchange={(e) => void handleColor(calendar, e.currentTarget.value)}
                aria-label={`Colour of ${calendar.summary}`}
            />
            <input
                type="text"
                class="name"
                value={calendar.summary}
                disabled={!calendar.writable || busy}
                onchange={(e) => void handleRename(calendar, e.currentTarget.value)}
                aria-label={`Name of ${calendar.summary}`}
            />
            <span class="kind">
                {kindLabel(calendar)}{#if !calendar.writable} · read-only{/if}
            </span>
            {#if calendar.color}
                <button
                    class="icon"
                    disabled={busy}
                    title="Clear colour"
                    aria-label={`Clear colour of ${calendar.summary}`}
                    onclick={() => void handleClearColor(calendar)}
                >
                    ×
                </button>
            {/if}
            <button
                class="icon"
                disabled={!calendar.writable || busy}
                title="Delete list"
                aria-label={`Delete ${calendar.summary}`}
                onclick={() => void handleDelete(calendar)}
            >
                🗑
            </button>
        </li>
    {/each}
</ul>
{#if calendars.length === 0}
    <p class="empty">No lists on the server yet.</p>
{/if}

<div class="add-row">
    <input
        type="text"
        bind:value={newName}
        placeholder="New list name"
        disabled={busy}
        onkeydown={(e) => {
            if (e.key === 'Enter') void handleCreate();
        }}
    />
    <select bind:value={newComponent} disabled={busy} aria-label="List kind">
        <option value="VTODO">Tasks</option>
        <option value="VEVENT">Events</option>
    </select>
    <button disabled={busy || !newName.trim()} onclick={() => void handleCreate()}>Add</button>
</div>

<style>
    .calendars {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: 2px;
    }

    .row {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 4px 0;
    }

    .colour {
        width: 22px;
        height: 22px;
        padding: 0;
        border: none;
        background: transparent;
        cursor: pointer;
    }

    .colour:disabled {
        opacity: 0.5;
        cursor: default;
    }

    .name {
        flex: 1;
        min-width: 0;
        background: transparent;
        border: 1px solid transparent;
        border-radius: 4px;
        color: var(--fg);
        padding: 4px 6px;
        font: inherit;
    }

    .name:hover:not(:disabled),
    .name:focus {
        border-color: var(--border);
        outline: none;
    }

    .name:disabled {
        opacity: 0.6;
    }

    .kind {
        font-size: 11px;
        color: var(--fg-dim);
        white-space: nowrap;
    }

    .icon {
        background: none;
        border: none;
        color: var(--fg-dim);
        cursor: pointer;
        padding: 2px 6px;
        border-radius: 4px;
        font-size: 13px;
    }

    .icon:hover:not(:disabled) {
        background: var(--bg-surface-hover);
        color: var(--fg);
    }

    .icon:disabled {
        opacity: 0.5;
        cursor: default;
    }

    .empty {
        color: var(--fg-dim);
        font-size: 13px;
        margin: 0;
    }

    .add-row {
        display: flex;
        gap: 8px;
        margin-top: 10px;
        padding-top: 16px;
        border-top: 1px solid var(--border);
    }

    .add-row input[type='text'] {
        flex: 1;
        min-width: 0;
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-radius: 4px;
        color: var(--fg);
        padding: 6px 8px;
        font: inherit;
    }

    .add-row select {
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-radius: 4px;
        color: var(--fg);
        padding: 6px 8px;
        font: inherit;
    }

    .add-row button {
        background: var(--accent, #3b82f6);
        border: none;
        color: #fff;
        border-radius: 4px;
        padding: 6px 14px;
        cursor: pointer;
        font: inherit;
    }

    .add-row button:disabled {
        opacity: 0.6;
        cursor: default;
    }
</style>
