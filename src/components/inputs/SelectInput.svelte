<script lang="ts">
    import { clickOutside } from '../../lib/clickOutside';

    interface Option {
        label: string;
        value: string;
        color?: string;
    }

    interface Props {
        value: string;
        onchange: (val: string) => void;
        options: Option[];
        class?: string;
        style?: string;
        disabled?: boolean;
        variant?: 'default' | 'bare';
        ariaLabel?: string;
    }

    let {
        value,
        onchange,
        options,
        class: className = '',
        style = '',
        disabled = false,
        variant = 'default',
        ariaLabel,
    }: Props = $props();

    let open = $state(false);
    let selectedOption = $derived(options.find(o => o.value === value));

    function handleSelect(o: Option, e: Event) {
        e.stopPropagation();
        onchange(o.value);
        open = false;
    }
</script>

<div
    class={`select-input ${className}`.trim()}
    {style}
    {@attach clickOutside(() => { open = false; }, { enabled: open })}
>
    <button
        type="button"
        class="selector-input"
        class:is-bare={variant === 'bare'}
        {disabled}
        aria-label={ariaLabel}
        onclick={() => { if (!disabled) open = !open; }}
    >
        {#if selectedOption?.color}
            <span
                aria-hidden="true"
                class="swatch"
                style:background={selectedOption.color}
            ></span>
        {/if}
        <span style:color={selectedOption?.color}>
            {selectedOption ? selectedOption.label : value}
        </span>
        <span class="material-symbols-outlined arrow">
            arrow_drop_down
        </span>
    </button>
    {#if open}
        <div class="dropdown">
            {#each options as o (o.value)}
                <button
                    type="button"
                    onclick={(e) => handleSelect(o, e)}
                    class="option"
                    class:is-selected={value === o.value}
                    style:color={o.color}
                >
                    {#if o.color}
                        <span
                            aria-hidden="true"
                            class="swatch"
                            style:background={o.color}
                        ></span>
                    {/if}
                    {o.label}
                </button>
            {/each}
        </div>
    {/if}
</div>

<style>
    .select-input {
        position: relative;
    }

    .selector-input {
        padding: 4px 8px;
        border: 1px solid var(--border);
        border-radius: 4px;
        background: var(--bg-surface);
        color: var(--fg);
        cursor: pointer;
        min-height: 24px;
        display: flex;
        align-items: center;
        gap: 4px;
        width: 100%;
        text-align: left;
        font-family: inherit;
        font-size: inherit;
    }

    .selector-input:disabled {
        cursor: not-allowed;
        opacity: 0.5;
    }

    .selector-input.is-bare {
        padding: 2px 0;
        border: none;
        background: transparent;
        gap: 6px;
    }

    .selector-input.is-bare:hover:not(:disabled) {
        color: var(--fg);
    }

    .swatch {
        width: 10px;
        height: 10px;
        border-radius: 50%;
        flex-shrink: 0;
    }

    .arrow {
        font-size: 16px;
        margin-left: auto;
        opacity: 0.5;
    }

    .dropdown {
        position: absolute;
        top: 100%;
        left: 0;
        right: 0;
        z-index: 1001;
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-radius: 4px;
        max-height: 200px;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        margin-top: 4px;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
    }

    .option {
        padding: 6px 8px;
        cursor: pointer;
        display: flex;
        align-items: center;
        gap: 6px;
        width: 100%;
        border: none;
        text-align: left;
        color: inherit;
        font-family: inherit;
        font-size: inherit;
        background: transparent;
    }

    .option.is-selected {
        background: var(--accent-soft);
    }
</style>
