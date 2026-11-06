<script lang="ts">
    import { onDestroy } from 'svelte';
    import { createDebouncedWriter } from '../../lib/asyncState.svelte';

    interface Props {
        value: string;
        onchange: (value: string) => void;
        disabled?: boolean;
    }

    let { value, onchange, disabled = false }: Props = $props();

    let element = $state<HTMLTextAreaElement | null>(null);
    let localValue = $state(value);
    let focused = $state(false);

    const writer = createDebouncedWriter<string>((next) => onchange(next), 400);

    $effect(() => {
        if (!focused && value !== localValue) localValue = value;
    });

    function autoGrow(node: HTMLTextAreaElement) {
        function resize(): void {
            node.style.height = 'auto';
            node.style.height = `${node.scrollHeight}px`;
        }
        resize();
        return { update: resize };
    }

    function handleInput() {
        writer.schedule(localValue);
    }

    function handleFocus() {
        focused = true;
    }

    function handleBlur() {
        focused = false;
        writer.flush();
    }

    onDestroy(() => writer.flush());
</script>

<textarea
    bind:this={element}
    bind:value={localValue}
    use:autoGrow={localValue}
    {disabled}
    rows="1"
    class="inspector-title-input"
    placeholder="Untitled"
    aria-label="Title"
    spellcheck="false"
    oninput={handleInput}
    onfocus={handleFocus}
    onblur={handleBlur}
></textarea>
