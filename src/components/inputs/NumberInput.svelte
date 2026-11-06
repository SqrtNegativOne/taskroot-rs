<script lang="ts">
    interface Props {
        value: number | string;
        onchange: (val: number) => void;
        min?: number;
        max?: number;
        class?: string;
        style?: string;
        disabled?: boolean;
        variant?: 'default' | 'bare';
        ariaLabel?: string;
        placeholder?: string;
    }

    let {
        value,
        onchange,
        min,
        max,
        class: className = '',
        style = '',
        disabled = false,
        variant = 'default',
        ariaLabel,
        placeholder,
    }: Props = $props();

    const inputStyle = $derived(
        variant === 'bare'
            ? `background: transparent; color: var(--fg); border: none; padding: 2px 0; width: 56px; font-family: inherit; font-size: inherit; cursor: ${disabled ? 'not-allowed' : 'text'}; opacity: ${disabled ? 0.5 : 1};`
            : `background: var(--bg-surface); color: var(--fg); border: 1px solid var(--border); border-radius: 4px; padding: 4px 8px; width: 80px;`,
    );

    function handleChange(e: Event & { currentTarget: EventTarget & HTMLInputElement }) {
        const val = Number(e.currentTarget.value);
        onchange(Number.isNaN(val) ? 0 : val);
    }
</script>

<input
    type="number"
    {min}
    {max}
    {value}
    {placeholder}
    {disabled}
    aria-label={ariaLabel}
    onchange={handleChange}
    class={className}
    style={`${inputStyle} ${style}`}
/>
