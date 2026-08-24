<script lang="ts">
    import { page } from '$app/state';
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { Routes } from '$lib/routes';

    let current = $derived(page.url.pathname === Routes.HOME ? 'plan' : page.url.pathname.substring(1));

    const defaultStages = [
        { key: 'plan', label: 'plan', href: Routes.HOME },
        { key: 'do', label: 'do', href: Routes.DO }
    ] as const;

    const isDefault = $derived(defaultStages.some(s => s.key === current) || current === 'settings');

    function navigate(href: typeof defaultStages[number]['href']): void {
        void goto(resolve(href));
    }
</script>

<nav class="stages" aria-label="Stages">
    {#each defaultStages as s, i (s.key)}
        <button
            class="stage {current === s.key ? 'is-current' : ''}"
            onclick={() => navigate(s.href)}
            style="background: transparent; border: none; cursor: pointer; color: inherit; padding: 0;"
        >
            <span class="stage-name">{s.label}</span>
        </button>
        {#if i < defaultStages.length - 1 || !isDefault}
            <span class="stage-sep">|</span>
        {/if}
    {/each}
    {#if !isDefault && current}
        <div class="stage is-current" style="display: flex;">
            <span class="stage-name">{current}</span>
        </div>
    {/if}
</nav>

<style>
    .stages {
        display: flex;
        align-items: center;
        gap: 8px;
    }
    .stage {
        padding: 2px 6px;
        border-radius: 4px;
        color: var(--fg-muted);
        text-transform: lowercase;
        font-weight: 500;
        transition: color 0.15s;
    }
    .stage:hover {
        color: var(--fg);
    }
    .stage.is-current {
        color: var(--fg);
        background: var(--bg-surface-hover, rgba(0,0,0,0.1));
    }
    .stage-name {
        font-family: inherit;
    }
    .stage-sep {
        color: var(--border-strong);
        opacity: 0.5;
        font-size: 0.9em;
    }
</style>
