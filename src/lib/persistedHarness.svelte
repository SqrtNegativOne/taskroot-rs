<script lang="ts">
    import { untrack } from 'svelte';
    import { persistState } from './persisted.svelte';

    let {
        storageKey,
        fallback = [],
        debounceMs = 400,
    }: {
        storageKey: string;
        fallback?: string[];
        debounceMs?: number;
    } = $props();

    let value = $state<string[]>(untrack(() => fallback));

    persistState(
        untrack(() => storageKey),
        () => value,
        (stored) => {
            value = stored;
        },
        {
            debounceMs: untrack(() => debounceMs),
            isValid: (stored): stored is string[] => Array.isArray(stored),
        },
    );
</script>

<div data-testid="value">{value.join(',')}</div>
<button onclick={() => { value = [...value, 'added']; }}>add</button>
