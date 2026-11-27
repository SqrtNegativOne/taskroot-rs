<script lang="ts">
    import type { ColourScheme } from '../bindings/ColourScheme.generated';
    import { notifyEventThemeChanged } from './eventTheme.svelte';

    let { scheme }: { scheme?: ColourScheme } = $props();

    $effect(() => {
        document.documentElement.dataset.theme = scheme ?? 'slate';
        notifyEventThemeChanged();
        return () => { delete document.documentElement.dataset.theme; };
    });
</script>
