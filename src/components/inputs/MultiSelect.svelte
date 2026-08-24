<script lang="ts">
    interface Option {
        value: string;
        label: string;
    }

    interface Props {
        options: Option[];
        selected: string[];
        onchange: (selected: string[]) => void;
        class?: string;
        style?: string;
    }

    let { options = [], selected = [], onchange, class: className = '', style = '' }: Props = $props();

    function toggleOption(val: string) {
        if (selected.includes(val)) {
            onchange(selected.filter(v => v !== val));
        } else {
            onchange([...selected, val]);
        }
    }
</script>

<div class={className} style={`display: flex; flex-direction: column; gap: 6px; ${style}`}>
    {#each options as opt}
        <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; color: var(--fg, inherit); font-size: 14px;">
            <input 
                type="checkbox" 
                checked={selected.includes(opt.value)}
                onchange={() => toggleOption(opt.value)}
                style="margin: 0; cursor: pointer;"
            />
            {opt.label}
        </label>
    {/each}
</div>
