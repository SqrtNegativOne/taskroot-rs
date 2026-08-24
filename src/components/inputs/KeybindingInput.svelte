<script lang="ts">
    interface Props {
        value: string;
        onchange: (val: string) => void;
        placeholder?: string;
        class?: string;
        style?: string;
    }

    let { value, onchange, placeholder = 'Press a key combination...', class: className = '', style = '' }: Props = $props();

    function handleKeyDown(e: KeyboardEvent) {
        if (e.key === 'Tab') return;
        
        e.preventDefault();
        
        const keys = [];
        if (e.ctrlKey) keys.push('Ctrl');
        if (e.metaKey) keys.push('Meta');
        if (e.altKey) keys.push('Alt');
        if (e.shiftKey) keys.push('Shift');
        
        if (!['Control', 'Meta', 'Alt', 'Shift'].includes(e.key)) {
            let key = e.key;
            if (key === ' ') key = 'Space';
            keys.push(key.length === 1 ? key.toUpperCase() : key);
        }
        
        if (keys.length > 0) {
            onchange(keys.join('+'));
        }
    }
</script>

<input
    type="text"
    {value}
    {placeholder}
    onkeydown={handleKeyDown}
    class={className}
    style={`background: var(--bg-input, transparent); color: var(--fg, inherit); border: 1px solid var(--border, #ccc); border-radius: 4px; padding: 6px 10px; cursor: text; box-sizing: border-box; width: 100%; ${style}`}
    readonly
/>
