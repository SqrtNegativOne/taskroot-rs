<script lang="ts">
    interface Props {
        value: string;
        onchange: (val: string) => void;
        class?: string;
        style?: string;
        disabled?: boolean;
    }

    let { value, onchange, class: className = '', style = '', disabled = false }: Props = $props();

    function handleChange(e: Event & { currentTarget: EventTarget & HTMLInputElement }) {
        onchange(e.currentTarget.value);
    }
</script>

<input
    type="time"
    {value}
    onchange={handleChange}
    {disabled}
    class={className}
    style={`background: var(--bg-surface); color: var(--fg); border: 1px solid var(--border); border-radius: 4px; padding: 4px 8px; cursor: ${disabled ? 'not-allowed' : 'text'}; opacity: ${disabled ? 0.5 : 1}; ${style}`}
/>
