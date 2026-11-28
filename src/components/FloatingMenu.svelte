<script lang="ts">
    import { Popover } from 'bits-ui';
    import type { Snippet } from 'svelte';

    let {
        align = 'left',
        minWidth = '280px',
        title,
        active = false,
        badge = 0,
        trigger,
        children,
        onOpenChange,
        triggerHeight,
    }: {
        align?: 'left' | 'right';
        minWidth?: string;
        title: string;
        active?: boolean;
        badge?: number;
        trigger: Snippet;
        children?: Snippet;
        onOpenChange?: (open: boolean) => void;
        triggerHeight?: string;
    } = $props();

    let open = $state(false);
</script>

<div class="filter-sort-container">
    <Popover.Root
        {open}
        onOpenChange={(next) => {
            open = next;
            onOpenChange?.(next);
        }}
    >
        <Popover.Trigger>
            {#snippet child({ props })}
                <button
                    {...props}
                    class="menu-trigger-button"
                    class:is-active={active || open}
                    {title}
                    aria-label={title}
                    style={triggerHeight ? `height: ${triggerHeight};` : ''}
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
    border-radius: 0;
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
</style>
