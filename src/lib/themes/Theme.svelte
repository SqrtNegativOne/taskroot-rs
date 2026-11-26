<script lang="ts">
    import type { ColourScheme } from '../bindings/ColourScheme.generated';

    let { scheme }: { scheme?: ColourScheme } = $props();

    $effect(() => {
        document.documentElement.dataset.theme = scheme ?? 'slate';
        return () => { delete document.documentElement.dataset.theme; };
    });
</script>
