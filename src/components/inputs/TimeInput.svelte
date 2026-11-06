<script lang="ts">
    interface Props {
        value: string;
        onchange: (val: string) => void;
        class?: string;
        style?: string;
        disabled?: boolean;
        variant?: 'default' | 'bare';
        ariaLabel?: string;
    }

    let {
        value,
        onchange,
        class: className = '',
        style = '',
        disabled = false,
        variant = 'default',
        ariaLabel,
    }: Props = $props();

    const inputStyle = $derived(
        variant === 'bare'
            ? `background: transparent; color: var(--fg); border: none; padding: 2px 0; font-family: inherit; font-size: inherit; cursor: ${disabled ? 'not-allowed' : 'text'}; opacity: ${disabled ? 0.5 : 1};`
            : `background: var(--bg-surface); color: var(--fg); border: 1px solid var(--border); border-radius: 4px; padding: 4px 8px; cursor: ${disabled ? 'not-allowed' : 'text'}; opacity: ${disabled ? 0.5 : 1};`,
    );

    function handleChange(e: Event & { currentTarget: EventTarget & HTMLInputElement }) {
        onchange(e.currentTarget.value);
    }
</script>

<input
    type="time"
    {value}
    {disabled}
    aria-label={ariaLabel}
    onchange={handleChange}
    class={className}
    style={`${inputStyle} ${style}`}
/>
