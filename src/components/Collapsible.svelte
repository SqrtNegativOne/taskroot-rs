<script lang="ts">
    import type { Snippet } from 'svelte';

    let {
        title,
        defaultOpen = false,
        badge,
        children
    }: {
        title: string;
        defaultOpen?: boolean;
        badge?: Snippet;
        children: Snippet;
    } = $props();

    // svelte-ignore state_referenced_locally
    let isOpen = $state(defaultOpen);

    function toggle() {
        isOpen = !isOpen;
    }
</script>

<div class="collapsible">
    <button class="collapsible-header" onclick={toggle} aria-expanded={isOpen}>
        <span class="collapsible-title">{title}</span>
        {#if badge}
            {@render badge()}
        {/if}
        <span class="collapsible-icon">
            {isOpen ? '▼' : '▶'}
        </span>
    </button>
    
    {#if isOpen}
        <div class="collapsible-content">
            {@render children()}
        </div>
    {/if}
</div>

<style>
    .collapsible {
        margin-bottom: 16px;
        border: 1px solid var(--border);
        border-radius: 8px;
        overflow: hidden;
    }

    .collapsible-header {
        display: flex;
        align-items: center;
        width: 100%;
        padding: 12px 16px;
        background: var(--bg-highlight);
        color: var(--fg);
        border: none;
        cursor: pointer;
        text-align: left;
    }

    .collapsible-title {
        flex: 1;
        font-weight: 600;
        text-transform: uppercase;
        font-size: 14px;
        letter-spacing: 0.05em;
    }

    .collapsible-icon {
        margin-left: 12px;
        font-size: 10px;
    }

    .collapsible-content {
        padding: 16px;
        background: var(--bg);
    }
</style>
