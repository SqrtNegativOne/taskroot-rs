<script lang="ts">
    interface Props {
        value: number | string;
        onchange: (val: number) => void;
        min?: number;
        max?: number;
        class?: string;
        style?: string;
    }

    let { value, onchange, min, max, class: className = '', style = '' }: Props = $props();

    function handleChange(e: Event & { currentTarget: EventTarget & HTMLInputElement }) {
        const val = Number(e.currentTarget.value);
        onchange(isNaN(val) ? 0 : val);
    }
</script>

<input
    type="number"
    {min}
    {max}
    {value}
    onchange={handleChange}
    class={className}
    style={`background: var(--bg-input); color: var(--fg); border: 1px solid var(--border); border-radius: 4px; padding: 4px 8px; width: 80px; ${style}`}
/>
