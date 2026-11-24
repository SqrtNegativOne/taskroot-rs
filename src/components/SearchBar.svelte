<script lang="ts">
    import Icon from './Icon.svelte';

    interface Props {
        value: string;
        onchange: (val: string) => void;
    }

    let { value, onchange }: Props = $props();

    function handleInput(e: Event & { currentTarget: EventTarget & HTMLInputElement }) {
        onchange(e.currentTarget.value);
    }

    function handleClear() {
        onchange("");
    }
</script>

<div class="task-pane-search">
    <span class="search-icon" class:is-hidden={value !== ''} aria-hidden="true">
        <Icon name="search" size={14} style="color: var(--fg-dimmer);" />
    </span>
    <input
        class="search-input"
        {value}
        oninput={handleInput}
        spellcheck="false"
        size={5}
    />
    {#if value}
        <button class="search-clear" onclick={handleClear} aria-label="clear">×</button>
    {/if}
</div>

<style>
    .task-pane-search {
        display: flex;
        align-items: center;
        gap: 6px;
        flex: 1;
        height: 28px;
        padding: 0 6px;
        background: var(--bg-surface);
        border: 1px solid var(--border);
        transition: border-color 120ms;
    }
    .task-pane-search:focus-within { border-color: var(--accent-line); }

    .search-icon {
        display: inline-flex;
        width: 20px; /* icon (14px) + the container's 6px gap */
        overflow: hidden;
        transition: opacity 150ms ease, width 150ms ease;
    }
    .search-icon.is-hidden { opacity: 0; width: 0; }

    .search-input {
        flex: 1;
        min-width: 0;
        background: none;
        border: 0;
        outline: none;
        color: var(--fg);
        font: inherit;
        padding: 0;
    }

    .search-clear {
        appearance: none;
        border: 0;
        background: none;
        color: var(--fg-dim);
        cursor: pointer;
        padding: 0 2px;
        font-size: 1.5em;
        line-height: 1;
    }
    .search-clear:hover { color: var(--fg); }
</style>
