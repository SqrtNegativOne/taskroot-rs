<script lang="ts">
    import { onMount, type Snippet } from 'svelte';
    import { events } from '../../lib/safeInvoke.svelte';

    let { children, preserve = false }: { children: Snippet; preserve?: boolean } = $props();
    let generation = $state(0);

    onMount(() => {
        let disposed = false;
        let unlisten: (() => void) | undefined;
        void events.accountChanged.listen(() => {
            if (!disposed) generation += 1;
        }).then((stop) => {
            if (disposed) stop();
            else unlisten = stop;
        });
        return () => { disposed = true; unlisten?.(); };
    });
</script>

{#if preserve}
    {@render children()}
{:else}
    {#key generation}
        {@render children()}
    {/key}
{/if}
