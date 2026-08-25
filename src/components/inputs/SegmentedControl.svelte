<script lang="ts">
    interface Option {
        value: string;
        label: string;
    }

    interface Props {
        options: Option[];
        selected: string;
        onchange: (val: string) => void;
        class?: string;
        style?: string;
    }

    let { options, selected, onchange, class: className = '', style = '' }: Props = $props();
</script>

<div class={className} style={`display: flex; background: var(--bg-input, #eee); border-radius: 6px; padding: 2px; width: fit-content; ${style}`}>
    {#each options as opt (opt.value)}
        <button
            type="button"
            onclick={() => onchange(opt.value)}
            style={`flex: 1; padding: 4px 12px; border: none; border-radius: 4px; cursor: pointer; font-size: 14px; background: ${selected === opt.value ? 'var(--accent, #007bff)' : 'transparent'}; color: ${selected === opt.value ? 'var(--bg, #fff)' : 'var(--fg, inherit)'}; transition: background 0.2s, color 0.2s; white-space: nowrap;`}
        >
            {opt.label}
        </button>
    {/each}
</div>
