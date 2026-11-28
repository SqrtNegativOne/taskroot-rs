<script lang="ts">
    import { commands, useAutoQuery } from '../../lib/safeInvoke.svelte';
    import { allEvents, allTasks } from '../../lib/queries';
    import type { AppEvent, AppTask } from '../../lib/domain';
    import { store } from '../../lib/store.svelte';
    import { toWallClockParts } from '../../lib/time';
    import Icon from '../../components/Icon.svelte';
    import SearchBar from '../../components/SearchBar.svelte';
    import InspectorPane from '../../components/inspector-pane/InspectorPane.svelte';
    import type { InspectorTarget } from '../../components/inspector-pane/state.svelte';
    import { groupByTag } from './grouping';

    let search = $state('');
    let inspectorState = $state<InspectorTarget | undefined>(undefined);
    let editingTag = $state<string | null>(null);
    let renameDraft = $state('');
    let renaming = $state(false);

    const tasksQuery = useAutoQuery(commands.queryTasks, allTasks);
    const eventsQuery = useAutoQuery(commands.queryEvents, allEvents);

    const groups = $derived(groupByTag(tasksQuery.data ?? [], eventsQuery.data ?? []));
    const filtered = $derived.by(() => {
        const needle = search.trim().toLowerCase();
        if (!needle) return groups;
        return groups.filter((group) => group.name.toLowerCase().includes(needle));
    });
    const trimmedDraft = $derived(renameDraft.trim());
    const draftValid = $derived(
        trimmedDraft.length > 0 && !trimmedDraft.includes(',') && !/[\r\n]/.test(trimmedDraft),
    );

    function countLabel(count: number, singular: string): string {
        const noun = count === 1 ? singular : `${singular}s`;
        return `${count} ${noun}`;
    }

    function eventWhen(event: AppEvent): string {
        const parts = toWallClockParts(event.startTime, event.timezone);
        if (!parts) return event.startTime;
        if (event.isAllDay || !parts.time) return parts.date;
        return `${parts.date} ${parts.time}`;
    }

    function taskMeta(task: AppTask): string {
        const parts: string[] = [];
        if (task.status) parts.push(task.status);
        const due = toWallClockParts(task.due, task.timezone);
        if (due) parts.push(`due ${due.date}`);
        return parts.join(' · ');
    }

    function openTask(task: AppTask): void {
        inspectorState = { type: 'task', id: task.id };
    }

    function openEvent(event: AppEvent): void {
        inspectorState = { type: 'event', id: event.id };
    }

    function focusAndSelect(node: HTMLInputElement) {
        node.focus();
        node.select();
    }

    function startRename(name: string): void {
        if (renaming) return;
        editingTag = name;
        renameDraft = name;
    }

    function cancelRename(): void {
        if (renaming) return;
        editingTag = null;
        renameDraft = '';
    }

    async function commitRename(from: string): Promise<void> {
        if (renaming || !draftValid) return;
        if (trimmedDraft === from) {
            cancelRename();
            return;
        }
        renaming = true;
        const result = await store.mutate('Failed to rename tag', () =>
            store.renameTag(from, trimmedDraft),
        );
        renaming = false;
        if (result.isOk()) {
            editingTag = null;
            renameDraft = '';
        }
    }

    function handleRenameKeydown(e: KeyboardEvent, from: string): void {
        if (e.key === 'Enter') {
            e.preventDefault();
            void commitRename(from);
        } else if (e.key === 'Escape') {
            e.preventDefault();
            cancelRename();
        }
    }
</script>

<main class="main tags-main">
    <div class="tags-scroll">
        <header class="tags-hd">
            <div class="tags-title">
                <span class="bracket">[</span>
                <span class="tags-title-label">tags</span>
                <span class="bracket">]</span>
                <span class="tags-count">{groups.length}</span>
            </div>
            <div class="tags-search">
                <SearchBar value={search} onchange={(value) => search = value} />
            </div>
        </header>

        {#if groups.length === 0}
            <div class="tags-empty">No tags yet.</div>
        {:else if filtered.length === 0}
            <div class="tags-empty">No tags match “{search.trim()}”.</div>
        {:else}
            <div class="tag-groups">
                {#each filtered as group (group.name)}
                    <section class="tag-group">
                        <header class="tag-group-hd">
                            <span class="tag-hash">#</span>
                            {#if editingTag === group.name}
                                <input
                                    type="text"
                                    class="tag-rename-input"
                                    bind:value={renameDraft}
                                    disabled={renaming}
                                    aria-label="Tag name"
                                    aria-invalid={!draftValid}
                                    spellcheck="false"
                                    use:focusAndSelect
                                    onkeydown={(e) => handleRenameKeydown(e, group.name)}
                                />
                                <button
                                    type="button"
                                    class="tag-icon-btn is-save"
                                    disabled={renaming || !draftValid}
                                    aria-label="Save tag name"
                                    title="Save"
                                    onclick={() => void commitRename(group.name)}
                                >
                                    <Icon name="check" size={14} />
                                </button>
                                <button
                                    type="button"
                                    class="tag-icon-btn"
                                    disabled={renaming}
                                    aria-label="Cancel rename"
                                    title="Cancel"
                                    onclick={cancelRename}
                                >
                                    <Icon name="close" size={14} />
                                </button>
                            {:else}
                                <h2 class="tag-group-name">{group.name}</h2>
                                <button
                                    type="button"
                                    class="tag-icon-btn tag-rename-btn"
                                    disabled={renaming}
                                    aria-label="Rename tag {group.name}"
                                    title="Rename tag"
                                    onclick={() => startRename(group.name)}
                                >
                                    <Icon name="edit" size={13} />
                                </button>
                            {/if}
                            <span class="tag-group-counts">
                                {countLabel(group.tasks.length, 'task')}
                                <span class="tag-sep">·</span>
                                {countLabel(group.events.length, 'event')}
                            </span>
                        </header>
                        <div class="tag-items">
                            {#each group.tasks as task (task.id)}
                                <button class="tag-item" onclick={() => openTask(task)}>
                                    <span class="tag-item-kind is-task">task</span>
                                    <span class="tag-item-title">{task.title}</span>
                                    <span class="tag-item-meta">{taskMeta(task)}</span>
                                </button>
                            {/each}
                            {#each group.events as event (event.id)}
                                <button class="tag-item" onclick={() => openEvent(event)}>
                                    <span class="tag-item-kind is-event">event</span>
                                    <span class="tag-item-title">{event.title}</span>
                                    <span class="tag-item-meta">{eventWhen(event)}</span>
                                </button>
                            {/each}
                        </div>
                    </section>
                {/each}
            </div>
        {/if}
    </div>

    <InspectorPane
        {inspectorState}
        onClose={() => { inspectorState = undefined; }}
        onConverted={(target) => { inspectorState = target; }}
    />
</main>

<style>
    .tags-main { position: relative; }
    .tags-scroll {
        flex: 1; min-height: 0; overflow-y: auto;
        padding: 16px 24px 48px;
        font-family: var(--sans); font-size: calc(14px * var(--font-scale, 1));
    }
    .tags-hd {
        display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between;
        gap: 12px; margin-bottom: 16px; padding-bottom: 12px;
        border-bottom: 1px dashed var(--border);
    }
    .tags-title { display: flex; align-items: baseline; gap: 4px; }
    .bracket { color: var(--accent); }
    .tags-title-label { letter-spacing: 0.1em; }
    .tags-count { margin-left: 2px; color: var(--fg-dim); font-size: 0.9em; }
    .tags-search { flex: 0 1 360px; min-width: 240px; }
    .tags-empty { padding: 24px 0; color: var(--fg-dim); }
    .tag-groups { display: flex; flex-direction: column; gap: 18px; }
    .tag-group { padding-bottom: 16px; border-bottom: 1px dashed var(--border-soft); }
    .tag-group-hd { display: flex; align-items: center; gap: 6px; min-height: 24px; margin-bottom: 8px; }
    .tag-hash { color: var(--accent-dim); }
    .tag-group-name { margin: 0; color: var(--fg); font-size: 1em; font-weight: 500; letter-spacing: 0.04em; }
    .tag-rename-input {
        flex: 0 1 220px; min-width: 96px; padding: 1px 6px;
        background: var(--bg-surface); border: 1px dashed var(--border); outline: none;
        color: var(--fg); font: inherit; font-weight: 500; letter-spacing: 0.04em;
    }
    .tag-rename-input:focus { border-style: solid; border-color: var(--accent-line); }
    .tag-rename-input[aria-invalid='true'] { border-color: var(--danger); }
    .tag-rename-input:disabled { opacity: 0.6; }
    .tag-icon-btn {
        display: inline-flex; align-items: center; justify-content: center;
        padding: 2px; border: 0; background: transparent;
        color: var(--fg-dim); cursor: pointer; line-height: 1;
    }
    .tag-icon-btn:hover:not(:disabled), .tag-icon-btn:focus-visible:not(:disabled) { color: var(--fg); }
    .tag-icon-btn.is-save:hover:not(:disabled) { color: var(--accent); }
    .tag-icon-btn:disabled { opacity: 0.35; cursor: default; }
    .tag-rename-btn { color: var(--fg-dimmer); opacity: 0.7; }
    .tag-group-hd:hover .tag-rename-btn { opacity: 1; }
    .tag-group-counts { margin-left: auto; color: var(--fg-dim); font-size: 0.82em; }
    .tag-sep { color: var(--fg-dimmer); }
    .tag-items { display: flex; flex-direction: column; }
    .tag-item {
        display: flex; align-items: baseline; gap: 10px; width: 100%;
        padding: 5px 8px; border: 0; border-left: 2px solid transparent;
        background: transparent; color: inherit; font: inherit; text-align: left; cursor: pointer;
    }
    .tag-item:hover { border-left-color: var(--accent-line); background: var(--bg-surface-hover); }
    .tag-item-kind { flex: 0 0 42px; color: var(--fg-dimmer); font-size: 0.72em; letter-spacing: 0.08em; text-transform: uppercase; }
    .tag-item-kind.is-task { color: var(--accent-dim); }
    .tag-item-kind.is-event { color: var(--tag-purple); }
    .tag-item-title { flex: 1; min-width: 0; overflow: hidden; color: var(--fg); text-overflow: ellipsis; white-space: nowrap; }
    .tag-item-meta { flex: 0 0 auto; color: var(--fg-dim); font-size: 0.82em; font-variant-numeric: tabular-nums; }
</style>
