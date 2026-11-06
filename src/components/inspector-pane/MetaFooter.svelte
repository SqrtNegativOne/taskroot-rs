<script lang="ts">
    import { formatTimestamp } from './format';

    interface Props {
        created?: string;
        modified?: string;
    }

    let { created, modified }: Props = $props();
</script>

{#if created || modified}
    <footer class="inspector-meta">
        {#if created}
            <span class="inspector-meta-item">
                <span class="material-symbols-outlined" aria-hidden="true">history</span>
                Created {formatTimestamp(created)}
            </span>
        {/if}
        {#if modified}
            <span class="inspector-meta-item">
                <span class="material-symbols-outlined" aria-hidden="true">update</span>
                Modified {formatTimestamp(modified)}
            </span>
        {/if}
    </footer>
{/if}
