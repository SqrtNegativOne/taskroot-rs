<script lang="ts">
    import { onMount } from 'svelte';

    interface Option {
        label: string;
        value: string;
    }

    interface Props {
        value: string;
        onchange: (val: string) => void;
        options: Option[];
        class?: string;
        style?: string;
    }

    let { value, onchange, options, class: className = '', style = '' }: Props = $props();

    let open = $state(false);
    let ref: HTMLDivElement | null = $state(null);
    let selectedOption = $derived(options.find(o => o.value === value));

    function handleClickOutside(e: PointerEvent) {
        if (ref && !(e.target instanceof Node && ref.contains(e.target))) {
            open = false;
        }
    }

    onMount(() => {
        document.addEventListener("pointerdown", handleClickOutside);
        return () => document.removeEventListener("pointerdown", handleClickOutside);
    });

    function handleSelect(o: Option, e: Event) {
        e.stopPropagation();
        onchange(o.value);
        open = false;
    }
</script>

<div bind:this={ref} style={`position: relative; ${style}`} class={className}>
    <button
        type="button"
        class="selector-input"
        style="padding: 4px 8px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg-app); color: var(--fg); cursor: pointer; min-height: 24px; display: flex; align-items: center; gap: 4px; width: 100%; text-align: left; font-family: inherit; font-size: inherit;"
        onclick={() => open = !open}
    >
        {selectedOption ? selectedOption.label : value}
        <span class="material-symbols-outlined" style="font-size: 16px; margin-left: auto; opacity: 0.5;">
            arrow_drop_down
        </span>
    </button>
    {#if open}
        <div style="position: absolute; top: 100%; left: 0; right: 0; z-index: 1001; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 4px; max-height: 200px; overflow-y: auto; display: flex; flex-direction: column; margin-top: 4px; box-shadow: 0 4px 12px rgba(0,0,0,0.2);">
            {#each options as o (o.value)}
                <button
                    type="button"
                    onclick={(e) => handleSelect(o, e)}
                    style={`padding: 6px 8px; cursor: pointer; display: flex; align-items: center; gap: 6px; background: ${value === o.value ? 'var(--accent-soft)' : 'transparent'}; width: 100%; border: none; text-align: left; color: inherit; font-family: inherit; font-size: inherit;`}
                >
                    {o.label}
                </button>
            {/each}
        </div>
    {/if}
</div>
