<script lang="ts">
    import { clickOutside } from '../../lib/clickOutside';

    interface Option {
        label: string;
        value: string;
    }

    interface Props {
        values: string[];
        onchange: (val: string[]) => void;
        options: Option[];
        class?: string;
        style?: string;
        placeholder?: string;
    }

    let { values = [], onchange, options, class: className = '', style = '', placeholder = 'Select...' }: Props = $props();

    let open = $state(false);

    function toggleSelect(o: Option, e: Event) {
        e.stopPropagation();
        if (values.includes(o.value)) {
            onchange(values.filter(v => v !== o.value));
        } else {
            onchange([...values, o.value]);
        }
    }
</script>

<div
    class={`multi-select ${className}`.trim()}
    {style}
    {@attach clickOutside(() => { open = false; }, { enabled: open })}
>
    <button type="button" class="selector-input" onclick={() => open = !open}>
        {#if values.length > 0}
            <div class="values">
                {#each values as v (v)}
                    {@const opt = options.find(o => o.value === v)}
                    <span class="tag">
                        {opt ? opt.label : v}
                    </span>
                {/each}
            </div>
        {:else}
            <span class="placeholder">{placeholder}</span>
        {/if}
        <span class="material-symbols-outlined expand-icon">
            expand_more
        </span>
    </button>
    {#if open}
        <div class="dropdown">
            {#if options.length === 0}
                <div class="empty">No options</div>
            {:else}
                {#each options as o (o.value)}
                    <label
                        title={o.label}
                        class="option"
                        class:is-selected={values.includes(o.value)}
                    >
                        <input
                            type="checkbox"
                            class="option-checkbox"
                            checked={values.includes(o.value)}
                            onchange={(e) => toggleSelect(o, e)}
                        />
                        <span class="option-label">{o.label}</span>
                    </label>
                {/each}
            {/if}
        </div>
    {/if}
</div>

<style>
    .multi-select {
        position: relative;
    }

    .selector-input {
        padding: 4px 8px;
        border: 1px solid var(--border);
        border-radius: 0;
        background: var(--bg);
        color: var(--fg);
        cursor: pointer;
        min-height: 28px;
        display: flex;
        align-items: center;
        gap: 4px;
        width: 100%;
        text-align: left;
        font-family: inherit;
        font-size: inherit;
    }

    .values {
        display: flex;
        gap: 4px;
        flex-wrap: wrap;
        flex: 1;
        overflow: hidden;
        max-height: 48px;
    }

    .tag {
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-radius: 0;
        padding: 0 4px;
        font-size: 0.9em;
        white-space: nowrap;
    }

    .placeholder {
        opacity: 0.6;
        flex: 1;
    }

    .expand-icon {
        font-size: 16px;
        margin-left: auto;
        opacity: 0.5;
    }

    .dropdown {
        position: absolute;
        top: calc(100% + 4px);
        right: 0;
        z-index: 1001;
        min-width: 100%;
        width: max-content;
        max-width: 320px;
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-radius: 0;
        max-height: 200px;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
    }

    .empty {
        padding: 6px 8px;
        opacity: 0.5;
        font-size: 0.9em;
    }

    .option {
        padding: 6px 8px;
        cursor: pointer;
        display: flex;
        align-items: center;
        gap: 6px;
        width: 100%;
        text-align: left;
        color: inherit;
        margin: 0;
        transition: background 0.1s ease;
        overflow: hidden;
    }

    .option:hover,
    .option.is-selected {
        background: var(--bg-surface-hover);
    }

    .option-checkbox {
        margin: 0;
        cursor: pointer;
        flex: 0 0 auto;
    }

    .option-label {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        min-width: 0;
    }
</style>
