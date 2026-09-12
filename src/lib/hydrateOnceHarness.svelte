<script lang="ts">
    import { untrack } from 'svelte';
    import { hydrateOnce } from './asyncState.svelte';

    let {
        load,
        onApplied,
    }: {
        load: () => Promise<string>;
        onApplied: (value: string) => void;
    } = $props();

    hydrateOnce(
        untrack(() => load),
        untrack(() => onApplied),
    );
</script>
