<script lang="ts">
    import Collapsible from '../../components/Collapsible.svelte';
    import Stopwatch from './stopwatch/Stopwatch.svelte';

    import { useTauriQuery } from '../../lib/safeInvoke.svelte';
    import type { AppTask } from '../../lib/domain';

    let isBreak = $state(false);
    let showRestOverride = $state(false);

    let tasksQuery = useTauriQuery<AppTask[]>('query_tasks');
    let tasks = $derived(tasksQuery.data ?? []);

    $effect(() => {
        void tasksQuery.execute({ filters: [], sort: [], query: "" });
    });

    function handleBreakStatusChange(status: boolean) {
        isBreak = status;
        showRestOverride = status;
    }
</script>

<main class="do-main">
    <Stopwatch onBreakStatusChange={handleBreakStatusChange} />

    {#if showRestOverride}
        <div class="do-rest-container">
            <div style="text-align: center; margin: 16px 0;">
                <button
                    onclick={() => showRestOverride = false}
                    class="sw-btn"
                    style="padding: 6px 14px; font-size: 13px;"
                >
                    Back to Do
                </button>
            </div>
            <div class="rest-screen-stub" style="text-align: center; padding: 48px;">
                Rest Screen
            </div>
        </div>
    {:else}
        <div class="do-sections">
            {#if isBreak}
                <div style="text-align: center; margin-bottom: 16px;">
                    <button
                        onclick={() => showRestOverride = true}
                        class="sw-btn"
                        style="padding: 6px 14px; font-size: 13px;"
                    >
                        Go to Rest Screen
                    </button>
                </div>
            {/if}

            <Collapsible title="distraction log" defaultOpen={true}>
                {#snippet badge()}
                    <span class="badge-count">0 entries</span>
                {/snippet}
                <div class="stub-content">Distraction Log Stub</div>
            </Collapsible>

            <Collapsible title="current tasks" defaultOpen={false}>
                {#snippet badge()}
                    <span class="badge-count">{tasks.length} tasks</span>
                {/snippet}
                <div class="stub-content">Kanban Stub</div>
            </Collapsible>

            <Collapsible title="tips" defaultOpen={false}>
                {#snippet badge()}
                    <span class="badge-count">0 tips</span>
                {/snippet}
                <div class="stub-content">Tips Stub</div>
            </Collapsible>

            <Collapsible title="notes" defaultOpen={false}>
                {#snippet badge()}
                    <span class="badge-count">0 notes</span>
                {/snippet}
                <div class="stub-content">Notes Stub</div>
            </Collapsible>
        </div>
    {/if}
</main>

<style>
    .do-main {
        display: flex;
        flex-direction: column;
        overflow-y: auto;
        height: 100%;
        color: var(--fg);
    }

    .do-sections {
        display: flex;
        flex-direction: column;
        padding: 0 24px 64px;
        max-width: 1400px;
        margin: 0 auto;
        width: 100%;
        margin-top: 24px;
    }

    .badge-count {
        font-size: 12px;
        color: var(--fg-dim);
        background: var(--bg-surface);
        padding: 2px 8px;
        border-radius: 12px;
    }

    .sw-btn {
        background: var(--bg-surface);
        color: var(--fg);
        border: 1px solid var(--border);
        border-radius: 4px;
        cursor: pointer;
    }

    .sw-btn:hover {
        background: var(--bg-surface-hover);
    }

    .stub-content {
        color: var(--fg-dim);
        font-style: italic;
    }
</style>
