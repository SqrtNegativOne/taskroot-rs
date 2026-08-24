<script lang="ts">
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { Routes } from '$lib/routes';
    import Icon from '../Icon.svelte';

    let dropdownOpen = $state(false);
    let dropdownRef: HTMLDivElement | null = $state(null);

    const STAGE_ROUTES = {
        wrap: Routes.WRAP,
        graph: Routes.GRAPH,
        stats: Routes.STATS,
        recap: Routes.RECAP,
        docs: Routes.DOCS,
        dev: Routes.DEV
    } as const;

    type Screen = keyof typeof STAGE_ROUTES;
    const SCREENS = Object.keys(STAGE_ROUTES) as Screen[];

    function handleOutsideClick(e: PointerEvent) {
        if (dropdownRef && e.target instanceof Node && !dropdownRef.contains(e.target)) {
            dropdownOpen = false;
        }
    }

    onMount(() => {
        document.addEventListener("pointerdown", handleOutsideClick);
        return () => document.removeEventListener("pointerdown", handleOutsideClick);
    });

    function navigate(screen: Screen) {
        dropdownOpen = false;
        void goto(resolve(STAGE_ROUTES[screen]));
    }
</script>

<div class="more-screens-dropdown" bind:this={dropdownRef} style="position: relative;">
    <button
        class="stage dropdown-btn {dropdownOpen ? 'is-current' : ''}"
        onclick={() => dropdownOpen = !dropdownOpen}
        title="More screens"
        style="background: transparent; border: none; cursor: pointer; color: inherit; padding: 4px; display: flex; align-items: center;"
    >
        <Icon name="more_horiz" size={18} />
    </button>
    
    {#if dropdownOpen}
        <div class="dropdown-menu" style="position: absolute; top: 100%; left: 0; z-index: 1000; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 4px; padding: 4px 0; min-width: 120px; box-shadow: 0 4px 12px rgba(0,0,0,0.15); display: flex; flex-direction: column;">
            {#each SCREENS as screen}
                <button
                    class="dd-item"
                    onclick={() => navigate(screen)}
                    style="background: transparent; border: none; cursor: pointer; color: inherit; text-align: left; padding: 6px 12px; font-family: inherit; font-size: 0.9em; width: 100%; transition: background 0.15s;"
                >
                    <span class="stage-name">{screen}</span>
                </button>
            {/each}
        </div>
    {/if}
</div>

<style>
    .dd-item:hover {
        background: var(--bg-surface-hover, rgba(255,255,255,0.05));
    }
</style>
