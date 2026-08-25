<script lang="ts">
    import { onMount } from 'svelte';

    interface Option {
        label: string;
        value: string;
    }

    interface Props {
        values: string[];
        onchange: (val: string[]) => void;
        options: Option[];
        class?: string;
        style?: string;
        placeholder?: string;
    }

    let { values = [], onchange, options, class: className = '', style = '', placeholder = 'Select...' }: Props = $props();

    let open = $state(false);
    let ref: HTMLDivElement | null = $state(null);

    function handleClickOutside(e: PointerEvent) {
        if (ref && !(e.target instanceof Node && ref.contains(e.target))) {
            open = false;
        }
    }

    onMount(() => {
        document.addEventListener("pointerdown", handleClickOutside);
        return () => document.removeEventListener("pointerdown", handleClickOutside);
    });

    function toggleSelect(o: Option, e: Event) {
        e.stopPropagation();
        if (values.includes(o.value)) {
            onchange(values.filter(v => v !== o.value));
        } else {
            onchange([...values, o.value]);
        }
    }
</script>

<div bind:this={ref} style={`position: relative; ${style}`} class={className}>
    <button
        type="button"
        class="selector-input"
        style="padding: 4px 8px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg); color: var(--fg); cursor: pointer; min-height: 28px; display: flex; align-items: center; gap: 4px; width: 100%; text-align: left; font-family: inherit; font-size: inherit;"
        onclick={() => open = !open}
    >
        {#if values.length > 0}
            <div style="display: flex; gap: 4px; flex-wrap: wrap; flex: 1; overflow: hidden; max-height: 48px;">
                {#each values as v (v)}
                    {@const opt = options.find(o => o.value === v)}
                    <span style="background: var(--bg-surface); border: 1px solid var(--border); border-radius: 4px; padding: 0 4px; font-size: 0.9em; white-space: nowrap;">
                        {opt ? opt.label : v}
                    </span>
                {/each}
            </div>
        {:else}
            <span style="opacity: 0.6; flex: 1;">{placeholder}</span>
        {/if}
        <span class="material-symbols-outlined" style="font-size: 16px; margin-left: auto; opacity: 0.5;">
            expand_more
        </span>
    </button>
    {#if open}
        <div style="position: absolute; top: calc(100% + 4px); left: 0; right: 0; z-index: 1001; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 4px; max-height: 200px; overflow-y: auto; display: flex; flex-direction: column; box-shadow: 0 4px 12px rgba(0,0,0,0.2);">
            {#if options.length === 0}
                <div style="padding: 6px 8px; opacity: 0.5; font-size: 0.9em;">No options</div>
            {:else}
                {#each options as o (o.value)}
                    <label style={`padding: 6px 8px; cursor: pointer; display: flex; align-items: center; gap: 6px; width: 100%; border: none; text-align: left; color: inherit; font-family: inherit; font-size: inherit; background: ${values.includes(o.value) ? 'var(--bg-app)' : 'transparent'}; margin: 0; transition: background 0.1s ease;`}
                           onpointerenter={(e) => {
                               if (!values.includes(o.value)) e.currentTarget.style.background = 'var(--bg-app)';
                           }}
                           onpointerleave={(e) => {
                               if (!values.includes(o.value)) e.currentTarget.style.background = 'transparent';
                           }}>
                        <input 
                            type="checkbox" 
                            checked={values.includes(o.value)}
                            onchange={(e) => toggleSelect(o, e)}
                            style="margin: 0; cursor: pointer;"
                        />
                        {o.label}
                    </label>
                {/each}
            {/if}
        </div>
    {/if}
</div>
