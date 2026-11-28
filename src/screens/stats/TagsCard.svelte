<script lang="ts">
    import BentoCard from './BentoCard.svelte';
    import { formatDuration } from './metrics';
    import type { TagStat } from './types';

    interface Props {
        tags: readonly TagStat[];
        totalKnownTags: number;
    }

    let { tags, totalKnownTags }: Props = $props();

    const maxCount = $derived(
        Math.max(1, ...tags.map((tag) => tag.totalCount)),
    );

    function barWidth(count: number): string {
        return `${Math.max(4, Math.round((count / maxCount) * 100))}%`;
    }
</script>

<BentoCard title="tag breakdown" span="col-6" testId="card-tags">
    {#snippet badge()}
        <span class="badge-muted">{totalKnownTags} tags</span>
    {/snippet}

    {#if tags.length === 0}
        <div class="empty-state">No tagged tasks or events yet.</div>
    {:else}
        <div class="rank-list">
            {#each tags as tag (tag.name)}
                <div class="rank-row">
                    <div class="rank-row-head">
                        <span class="tag-name">
                            <span
                                class="tag-dot"
                                style:background={tag.color || 'var(--accent)'}
                            ></span>
                            #{tag.name}
                        </span>
                        <span class="rank-meta">
                            {#if tag.trackedSeconds > 0}
                                <span class="est-badge">{formatDuration(tag.trackedSeconds)}</span>
                            {/if}
                            {#if tag.taskCount > 0}
                                <span class="rank-sub">{tag.doneCount}/{tag.taskCount} done</span>
                            {/if}
                            <span class="rank-val">{tag.totalCount}</span>
                        </span>
                    </div>
                    <div class="bar-track">
                        <div
                            class="bar-fill"
                            style:width={barWidth(tag.totalCount)}
                            style:background={tag.color || 'var(--accent)'}
                        ></div>
                    </div>
                </div>
            {/each}
        </div>
    {/if}
</BentoCard>
