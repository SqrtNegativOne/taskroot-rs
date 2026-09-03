<script lang="ts">
    interface Props {
        value: string;
        onchange: (val: string) => void;
        class?: string;
        style?: string;
    }

    let { value, onchange, class: className = '', style = '' }: Props = $props();

    function handleChange(e: Event & { currentTarget: EventTarget & HTMLInputElement }) {
        onchange(e.currentTarget.value);
    }
</script>

<input
    type="time"
    {value}
    onchange={handleChange}
    class={className}
    style={`background: var(--bg-surface); color: var(--fg); border: 1px solid var(--border); border-radius: 4px; padding: 4px 8px; ${style}`}
/>
