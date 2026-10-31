<script lang="ts">
    import { Popover } from 'bits-ui';
    import type { Snippet } from 'svelte';

    let {
        align = 'left',
        minWidth = '280px',
        title,
        active = false,
        badge = 0,
        addLabel,
        onadd,
        trigger,
        children,
    }: {
        align?: 'left' | 'right';
        minWidth?: string;
        title: string;
        active?: boolean;
        badge?: number;
        addLabel?: string;
        onadd?: () => void;
        trigger: Snippet;
        children?: Snippet;
    } = $props();

    let open = $state(false);
</script>

<div class="filter-sort-container">
    <Popover.Root bind:open>
        <Popover.Trigger>
            {#snippet child({ props })}
                <button
                    {...props}
                    class="menu-trigger-button"
                    class:is-active={active || open}
                    {title}
                >
                    {@render trigger()}
                    {#if badge > 0}
                        <span class="menu-trigger-badge">{badge}</span>
                    {/if}
                </button>
            {/snippet}
        </Popover.Trigger>

        <Popover.Portal>
            <Popover.Content
                sideOffset={8}
                align={align === 'left' ? 'start' : 'end'}
                style="min-width: {minWidth};"
            >
                {#snippet child({ props, wrapperProps })}
                    <div {...wrapperProps}>
                        <div {...props} class="shared-floating-menu">
                            {@render children?.()}
                            {#if addLabel && onadd}
                                <button class="filter-add-button" onclick={onadd}>
                                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
                                    {addLabel}
                                </button>
                            {/if}
                        </div>
                    </div>
                {/snippet}
            </Popover.Content>
        </Popover.Portal>
    </Popover.Root>
</div>

<style>
.filter-sort-container {
    display: flex;
    gap: 8px;
    align-items: center;
}

.shared-floating-menu {
    position: relative;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    background: var(--bg-surface);
    border-radius: 6px;
    border: 1px solid var(--border);
    box-shadow: 0 4px 12px rgba(0,0,0,0.2);
    animation: sharedFloatingMenuOpen 0.15s ease-out forwards;
    transform-origin: top center;
}

@keyframes sharedFloatingMenuOpen {
    from { opacity: 0; transform: scaleY(0.95); }
    to { opacity: 1; transform: scaleY(1); }
}

.menu-trigger-button {
    border: 1px solid var(--border);
    color: var(--fg);
    border-radius: 4px;
    padding: 4px 6px;
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
    background: transparent;
    transition: background 0.1s ease;
}

.menu-trigger-button.is-active,
.menu-trigger-button:hover {
    background: var(--bg-surface);
}

.menu-trigger-badge {
    font-size: 0.8em;
    font-weight: bold;
}

.filter-add-button {
    background: transparent;
    border: none;
    color: var(--fg);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 4px;
    align-self: flex-start;
    padding: 4px 4px;
    font-size: 0.9em;
    opacity: 0.8;
    transition: opacity 0.1s ease;
}

.filter-add-button:hover {
    opacity: 1;
}
</style>
