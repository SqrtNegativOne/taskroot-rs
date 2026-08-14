<script lang="ts">
    import { onMount } from 'svelte';
    import { ANIMATION_DELAY_MS, DateGridView } from './constants';

    let {
        titleLabel,
        today,
        view,
        setView,
        setAnchor,
        shift,
        filterMenu
    }: {
        titleLabel: string;
        today: Date;
        view: DateGridView;
        setView: (view: DateGridView) => void;
        setAnchor: (date: Date) => void;
        shift: (n: number) => void;
        filterMenu?: import('svelte').Snippet;
    } = $props();

    let showViewMenu = $state(false);
    let closingViewMenu = $state(false);
    let viewMenuRef = $state<HTMLDivElement | null>(null);

    function closeViewMenu() {
        closingViewMenu = true;
        setTimeout(() => {
            showViewMenu = false;
            closingViewMenu = false;
        }, ANIMATION_DELAY_MS);
    }

    onMount(() => {
        function handleClickOutside(e: PointerEvent) {
            if (
                e.target instanceof Node &&
                viewMenuRef &&
                !viewMenuRef.contains(e.target)
            ) {
                if (showViewMenu && !closingViewMenu) closeViewMenu();
            }
        }
        document.addEventListener("pointerdown", handleClickOutside);
        return () => { document.removeEventListener("pointerdown", handleClickOutside); };
    });
</script>

<header class="cal-hd">
    <div class="cal-hd-left">
        <span class="cal-hd-title">{titleLabel}</span>
    </div>
    <div class="cal-hd-right">
        <div class="cal-nav">
            <button
                class="cal-nav-btn"
                onclick={() => { shift(-1); }}
                aria-label="previous"
            >
                ◀
            </button>
            <button
                class="cal-nav-btn"
                onclick={() => { setAnchor(new Date(today)); }}
            >
                ◉
            </button>
            <button
                class="cal-nav-btn"
                onclick={() => { shift(1); }}
                aria-label="next"
            >
                ▶
            </button>
        </div>
        
        {#if filterMenu}
            {@render filterMenu()}
        {/if}

        <div style="position: relative;" bind:this={viewMenuRef}>
            <button
                onclick={() => {
                    if (showViewMenu) closeViewMenu();
                    else showViewMenu = true;
                }}
                title="View options"
                style="
                    background: {showViewMenu ? 'var(--bg-surface)' : 'transparent'};
                    border: 1px solid var(--border);
                    color: var(--fg);
                    border-radius: 4px;
                    padding: 4px 6px;
                    display: flex;
                    align-items: center;
                    gap: 4px;
                    cursor: pointer;
                "
            >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 3v18"/><path d="M15 3v18"/></svg>
            </button>
            
            {#if showViewMenu}
                <div
                    class="floating-menu"
                    class:is-closing={closingViewMenu}
                    style="
                        position: absolute;
                        top: calc(100% + 8px);
                        right: 0;
                        z-index: 1000;
                        display: flex;
                        flex-direction: column;
                        background: var(--bg-surface);
                        border-radius: 6px;
                        border: 1px solid var(--border);
                        box-shadow: 0 4px 12px rgba(0,0,0,0.2);
                        min-width: 120px;
                        overflow: hidden;
                    "
                >
                    {#each [DateGridView.Month, DateGridView.OneWeek, DateGridView.ThreeWeeks] as mode (mode)}
                        <button
                            onclick={() => {
                                setView(mode);
                                closeViewMenu();
                            }}
                            style="
                                padding: 8px 12px;
                                text-align: left;
                                background: {view === mode || (view === DateGridView.Week && mode === DateGridView.OneWeek) ? 'var(--accent-soft)' : 'transparent'};
                                color: {view === mode || (view === DateGridView.Week && mode === DateGridView.OneWeek) ? 'var(--accent)' : 'var(--fg)'};
                                border: none;
                                cursor: pointer;
                                font-size: 0.9em;
                                text-transform: capitalize;
                            "
                        >
                            {mode}
                        </button>
                    {/each}
                </div>
            {/if}
        </div>
    </div>
</header>
