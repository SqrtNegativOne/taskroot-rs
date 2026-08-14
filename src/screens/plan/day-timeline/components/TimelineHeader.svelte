<script lang="ts">
    import { onMount } from 'svelte';
    import { SvelteDate } from 'svelte/reactivity';

    let {
        viewDate,
        isToday,
        today,
        setTimelineDate,
        filterMenu,
        numDays,
        setNumDays,
    }: {
        viewDate: Date;
        isToday: boolean;
        today: Date;
        setTimelineDate: (d: Date) => void;
        filterMenu?: import('svelte').Snippet;
        numDays?: number;
        setNumDays?: (n: number) => void;
    } = $props();

    const NUM_DAYS_OPTIONS = [1, 2, 3, 4, 5, 6, 7];
    let showViewMenu = $state(false);
    let viewMenuRef = $state<HTMLDivElement | null>(null);

    function addDays(d: Date, days: number) {
        const nd = new SvelteDate(d);
        nd.setDate(nd.getDate() + days);
        return nd;
    }

    onMount(() => {
        function handleClickOutside(e: PointerEvent) {
            if (e.target instanceof Node && viewMenuRef && !viewMenuRef.contains(e.target)) {
                showViewMenu = false;
            }
        }
        document.addEventListener('pointerdown', handleClickOutside);
        return () => { document.removeEventListener('pointerdown', handleClickOutside); };
    });
</script>

<header class="cal-hd">
    <div class="cal-hd-left" style="display: flex; align-items: center; gap: 8px;">
        <span class="cal-hd-title" style="color: {isToday ? 'inherit' : 'var(--accent)'}">
            {Intl.DateTimeFormat('en-US', { weekday: 'long' }).format(viewDate)}
        </span>
    </div>
    <div class="cal-hd-right">
        <div class="cal-nav">
            <button
                class="cal-nav-btn"
                onclick={() => { setTimelineDate(addDays(viewDate, -1)); }}
                aria-label="previous"
            >
                ◀
            </button>
            <button class="cal-nav-btn" onclick={() => { setTimelineDate(today); }}>
                ◉
            </button>
            <button
                class="cal-nav-btn"
                onclick={() => { setTimelineDate(addDays(viewDate, 1)); }}
                aria-label="next"
            >
                ▶
            </button>
        </div>
        
        {#if filterMenu}
            {@render filterMenu()}
        {/if}

        {#if numDays !== undefined && setNumDays}
            <div style="position: relative;" bind:this={viewMenuRef}>
                <button
                    onclick={() => showViewMenu = !showViewMenu}
                    title="View columns"
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
                        {#each NUM_DAYS_OPTIONS as n (n)}
                            <button
                                onclick={() => {
                                    setNumDays(n);
                                    showViewMenu = false;
                                }}
                                style="
                                    padding: 8px 12px;
                                    text-align: left;
                                    background: {numDays === n ? 'var(--accent-soft)' : 'transparent'};
                                    color: {numDays === n ? 'var(--accent)' : 'var(--fg)'};
                                    border: none;
                                    cursor: pointer;
                                    font-size: 0.9em;
                                "
                            >
                                {n} day{n !== 1 ? 's' : ''}
                            </button>
                        {/each}
                    </div>
                {/if}
            </div>
        {/if}
    </div>
</header>
