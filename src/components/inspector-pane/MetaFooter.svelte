<script lang="ts">
    import { formatTimestamp } from './format';

    interface Props {
        created?: string;
        modified?: string;
        onDelete?: () => void;
        deleteLabel?: string;
        deleteDisabled?: boolean;
    }

    let { created, modified, onDelete, deleteLabel = 'Delete', deleteDisabled = false }: Props = $props();
</script>

{#if created || modified || onDelete}
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
        {#if onDelete}
            <button
                type="button"
                class="inspector-meta-delete"
                onclick={onDelete}
                disabled={deleteDisabled}
            >
                <span class="material-symbols-outlined" aria-hidden="true">delete</span>
                {deleteLabel}
            </button>
        {/if}
    </footer>
{/if}
