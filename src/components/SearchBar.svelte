<script lang="ts">
    import Icon from './Icon.svelte';

    interface Props {
        value: string;
        onchange: (val: string) => void;
        placeholder?: string;
    }

    let { value, onchange, placeholder = "" }: Props = $props();

    function handleInput(e: Event & { currentTarget: EventTarget & HTMLInputElement }) {
        onchange(e.currentTarget.value);
    }

    function handleClear() {
        onchange("");
    }
</script>

<div class="task-pane-search" style="display: flex; align-items: center; flex: 1;">
    <Icon name="search" size={14} style="color: var(--fg-dimmer);" />
    <input
        class="search-input"
        {value}
        oninput={handleInput}
        spellcheck="false"
        {placeholder}
        style="flex: 1;"
    />
    {#if value}
        <button
            class="search-clear"
            onclick={handleClear}
            aria-label="clear"
        >
            ×
        </button>
    {/if}
</div>
