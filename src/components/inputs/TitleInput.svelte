<script lang="ts">
    interface Props {
        value: string;
        onchange: (val: string) => void;
        placeholder?: string;
        class?: string;
        style?: string;
        autofocus?: boolean;
        disabled?: boolean;
    }

    let { 
        value, 
        onchange, 
        placeholder = 'Enter title...', 
        class: className = '', 
        style = '',
        autofocus = false,
        disabled = false
    }: Props = $props();

    function handleInput(e: Event & { currentTarget: EventTarget & HTMLInputElement }) {
        onchange(e.currentTarget.value);
    }
</script>

<!-- svelte-ignore a11y_autofocus -->
<input
    type="text"
    {value}
    {placeholder}
    oninput={handleInput}
    {autofocus}
    {disabled}
    class={className}
    style={`background: var(--bg-surface); color: var(--fg, inherit); border: 1px solid var(--border, #ccc); border-radius: 4px; padding: 8px 12px; font-size: 16px; width: 100%; box-sizing: border-box; outline: none; transition: border-color 0.2s; cursor: ${disabled ? 'not-allowed' : 'text'}; opacity: ${disabled ? 0.6 : 1}; ${style}`}
/>

<style>
    input:focus {
        border-color: var(--accent, #007bff);
    }
</style>
