<script lang="ts">
    import { formatFriendlyDate } from './format';

    interface Props {
        value?: string;
        placeholder: string;
        ariaLabel: string;
        disabled?: boolean;
        min?: string;
        max?: string;
        onchange: (value: string) => void;
        onclear?: () => void;
    }

    let {
        value = '',
        placeholder,
        ariaLabel,
        disabled = false,
        min,
        max,
        onchange,
        onclear,
    }: Props = $props();

    let label = $derived(value ? formatFriendlyDate(value) : placeholder);
</script>

<div
    class="inspector-date-field"
    class:is-empty={!value}
    class:is-disabled={disabled}
>
    <span class="inspector-date-label">{label}</span>
    <input
        type="date"
        class="inspector-date-native"
        {value}
        {min}
        {max}
        {disabled}
        aria-label={ariaLabel}
        onchange={(event) => onchange(event.currentTarget.value)}
    />
    {#if value && onclear && !disabled}
        <button
            type="button"
            class="inspector-clear"
            aria-label={`Clear ${ariaLabel}`}
            onclick={onclear}
        >
            <span class="material-symbols-outlined" style="font-size: 14px;">close</span>
        </button>
    {/if}
</div>
