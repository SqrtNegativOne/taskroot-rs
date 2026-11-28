<script lang="ts">
    import type { Snippet } from 'svelte';

    type BentoSpan = 'col-3' | 'col-4' | 'col-6' | 'col-8' | 'col-12';

    interface Props {
        title: string;
        span?: BentoSpan;
        badge?: Snippet;
        children: Snippet;
        testId?: string;
    }

    let { title, span = 'col-6', badge, children, testId }: Props = $props();
</script>

<section class={`bento-card ${span}`} data-testid={testId}>
    <header class="bento-card-hd">
        <h2 class="bento-card-title">{title}</h2>
        {#if badge}
            <div class="bento-card-badge">{@render badge()}</div>
        {/if}
    </header>
    <div class="bento-card-body">
        {@render children()}
    </div>
</section>
