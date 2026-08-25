<script lang="ts">
    interface Props {
        value: string;
        onchange: (val: string) => void;
        placeholder?: string;
        class?: string;
        style?: string;
        autofocus?: boolean;
    }

    let { 
        value, 
        onchange, 
        placeholder = 'Enter title...', 
        class: className = '', 
        style = '',
        autofocus = false
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
    class={className}
    style={`background: var(--bg-input, transparent); color: var(--fg, inherit); border: 1px solid var(--border, #ccc); border-radius: 4px; padding: 8px 12px; font-size: 16px; width: 100%; box-sizing: border-box; outline: none; transition: border-color 0.2s; ${style}`}
/>

<style>
    input:focus {
        border-color: var(--accent, #007bff);
    }
</style>
