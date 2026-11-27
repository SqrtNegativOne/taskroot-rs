<script lang="ts">
    import { commands, useAutoQuery } from '../../lib/safeInvoke.svelte';
    import { allEvents, allTasks } from '../../lib/queries';
    import type { AppEvent, AppTask } from '../../lib/domain';
    import SearchBar from '../../components/SearchBar.svelte';
    import InspectorPane from '../../components/inspector-pane/InspectorPane.svelte';
    import type { InspectorTarget } from '../../components/inspector-pane/state.svelte';
    import { groupByTag } from './grouping';

    let search = $state('');
    let inspectorState = $state<InspectorTarget | undefined>(undefined);

    const tasksQuery = useAutoQuery(commands.queryTasks, allTasks);
    const eventsQuery = useAutoQuery(commands.queryEvents, allEvents);

    const groups = $derived(groupByTag(tasksQuery.data ?? [], eventsQuery.data ?? []));
    const filtered = $derived.by(() => {
        const needle = search.trim().toLowerCase();
        if (!needle) return groups;
        return groups.filter((group) => group.name.toLowerCase().includes(needle));
    });

    function countLabel(count: number, singular: string): string {
        const noun = count === 1 ? singular : `${singular}s`;
        return `${count} ${noun}`;
    }

    function eventWhen(event: AppEvent): string {
        const day = event.startTime.slice(0, 10);
        if (event.isAllDay || event.startTime.length <= 10) return day;
        return `${day} ${event.startTime.slice(11, 16)}`;
    }

    function taskMeta(task: AppTask): string {
        const parts: string[] = [];
        if (task.status) parts.push(task.status);
        if (task.due) parts.push(`due ${task.due.slice(0, 10)}`);
        return parts.join(' · ');
    }

    function openTask(task: AppTask): void {
        inspectorState = { type: 'task', id: task.id };
    }

    function openEvent(event: AppEvent): void {
        inspectorState = { type: 'event', id: event.id };
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
                            <h2 class="tag-group-name">{group.name}</h2>
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
    .tags-main {
        position: relative;
    }

    .tags-scroll {
        flex: 1;
        min-height: 0;
        overflow-y: auto;
        padding: 16px 24px 48px;
        font-family: var(--sans);
        font-size: calc(14px * var(--font-scale, 1));
    }

    .tags-hd {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        margin-bottom: 16px;
        padding-bottom: 12px;
        border-bottom: 1px dashed var(--border);
    }

    .tags-title {
        display: flex;
        align-items: baseline;
        gap: 4px;
    }

    .bracket {
        color: var(--accent);
    }

    .tags-title-label {
        letter-spacing: 0.1em;
    }

    .tags-count {
        margin-left: 2px;
        color: var(--fg-dim);
        font-size: 0.9em;
    }

    .tags-search {
        flex: 0 1 360px;
        min-width: 240px;
    }

    .tags-empty {
        padding: 24px 0;
        color: var(--fg-dim);
    }

    .tag-groups {
        display: flex;
        flex-direction: column;
        gap: 18px;
    }

    .tag-group {
        padding-bottom: 16px;
        border-bottom: 1px dashed var(--border-soft);
    }

    .tag-group-hd {
        display: flex;
        align-items: baseline;
        gap: 6px;
        margin-bottom: 8px;
    }

    .tag-hash {
        color: var(--accent-dim);
    }

    .tag-group-name {
        margin: 0;
        color: var(--fg);
        font-size: 1em;
        font-weight: 500;
        letter-spacing: 0.04em;
    }

    .tag-group-counts {
        margin-left: auto;
        color: var(--fg-dim);
        font-size: 0.82em;
    }

    .tag-sep {
        color: var(--fg-dimmer);
    }

    .tag-items {
        display: flex;
        flex-direction: column;
    }

    .tag-item {
        display: flex;
        align-items: baseline;
        gap: 10px;
        width: 100%;
        padding: 5px 8px;
        border: 0;
        border-left: 2px solid transparent;
        background: transparent;
        color: inherit;
        font: inherit;
        text-align: left;
        cursor: pointer;
    }

    .tag-item:hover {
        border-left-color: var(--accent-line);
        background: var(--bg-surface-hover);
    }

    .tag-item-kind {
        flex: 0 0 42px;
        color: var(--fg-dimmer);
        font-size: 0.72em;
        letter-spacing: 0.08em;
        text-transform: uppercase;
    }

    .tag-item-kind.is-task {
        color: var(--accent-dim);
    }

    .tag-item-kind.is-event {
        color: var(--tag-purple);
    }

    .tag-item-title {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        color: var(--fg);
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .tag-item-meta {
        flex: 0 0 auto;
        color: var(--fg-dim);
        font-size: 0.82em;
        font-variant-numeric: tabular-nums;
    }
</style>
