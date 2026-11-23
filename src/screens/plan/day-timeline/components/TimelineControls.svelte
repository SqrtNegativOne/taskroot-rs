<script lang="ts">
    import { onMount, type Snippet } from 'svelte';
    import { SvelteDate } from 'svelte/reactivity';
    import { NUM_DAYS_OPTIONS } from '../constants';

    let {
        viewDate,
        today,
        setTimelineDate,
        filterMenu,
        numDays,
        setNumDays,
        keepOpen = false,
    }: {
        viewDate: Date;
        today: Date;
        setTimelineDate: (d: Date) => void;
        filterMenu?: Snippet;
        numDays?: number;
        setNumDays?: (n: number) => void;
        keepOpen?: boolean;
    } = $props();

    const CLOSE_DELAY_MS = 600;

    let open = $state(false);
    let hovering = $state(false);
    let focused = $state(false);
    let pinned = $state(false);
    let showViewMenu = $state(false);
    let root = $state<HTMLDivElement | null>(null);
    let viewMenuRef = $state<HTMLDivElement | null>(null);

    $effect(() => {
        const stayOpen = hovering || focused || pinned || keepOpen || showViewMenu;
        if (stayOpen) {
            open = true;
            return;
        }
        if (!open) return;
        const timer = setTimeout(() => { open = false; }, CLOSE_DELAY_MS);
        return () => { clearTimeout(timer); };
    });

    $effect(() => {
        if (!open) showViewMenu = false;
    });

    function addDays(d: Date, days: number) {
        const nd = new SvelteDate(d);
        nd.setDate(nd.getDate() + days);
        return nd;
    }

    function onFocusOut(e: FocusEvent) {
        if (e.relatedTarget instanceof Node && root?.contains(e.relatedTarget)) return;
        focused = false;
    }

    function onKeyDown(e: KeyboardEvent) {
        if (e.key !== 'Escape') return;
        pinned = false;
        showViewMenu = false;
        open = false;
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

<div
    class="timeline-controls"
    class:is-open={open}
    class:is-pinned={pinned}
    bind:this={root}
    onpointerenter={() => { hovering = true; open = true; }}
    onpointerleave={() => { hovering = false; }}
    onfocusin={() => {
        // Keyboard focus opens and holds the panel; a mouse click is already
        // covered by hover (and can pin it), so it should still auto-collapse.
        if (hovering) return;
        focused = true;
        open = true;
    }}
    onfocusout={onFocusOut}
    onkeydown={onKeyDown}
>
    <button
        class="timeline-eye"
        type="button"
        aria-label="Timeline controls"
        aria-expanded={open}
        onclick={() => { pinned = !pinned; }}
    >
        <svg
            class="timeline-eye-icon"
            viewBox="0 0 28 16"
            width="28"
            height="16"
            fill="none"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M1 8c3.6-4.6 8.6-7 13-7s9.4 2.4 13 7c-3.6 4.6-8.6 7-13 7S4.6 12.6 1 8Z" />
            <circle class="timeline-eye-pupil" cx="14" cy="8" r="3.2" />
        </svg>
    </button>

    <div class="timeline-eye-panel" inert={!open}>
        <div class="timeline-controls-group">
            <button
                class="timeline-control-btn"
                onclick={() => { setTimelineDate(addDays(viewDate, -1)); }}
                aria-label="Previous day"
                title="Previous day"
            >
                ◀
            </button>
            <button
                class="timeline-control-btn"
                onclick={() => { setTimelineDate(today); }}
                aria-label="Today"
                title="Today"
            >
                ◉
            </button>
            <button
                class="timeline-control-btn"
                onclick={() => { setTimelineDate(addDays(viewDate, 1)); }}
                aria-label="Next day"
                title="Next day"
            >
                ▶
            </button>
        </div>

        {#if filterMenu}
            <div class="timeline-controls-group">
                {@render filterMenu()}
            </div>
        {/if}

        {#if numDays !== undefined && setNumDays}
            <div class="timeline-controls-group timeline-view" bind:this={viewMenuRef}>
                <button
                    class="timeline-control-btn"
                    class:is-active={showViewMenu}
                    onclick={() => { showViewMenu = !showViewMenu; }}
                    title="View columns"
                    aria-label="View columns"
                >
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 3v18"/><path d="M15 3v18"/></svg>
                </button>
                {#if showViewMenu}
                    <div class="timeline-view-menu">
                        {#each NUM_DAYS_OPTIONS as n (n)}
                            <button
                                class:is-active={numDays === n}
                                onclick={() => { setNumDays(n); showViewMenu = false; }}
                            >
                                {n} day{n !== 1 ? 's' : ''}
                            </button>
                        {/each}
                    </div>
                {/if}
            </div>
        {/if}
    </div>
</div>
