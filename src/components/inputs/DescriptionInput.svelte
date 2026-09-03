<script lang="ts">
    interface Props {
        value: string | undefined | null;
        onchange: (val: string) => void;
        disabled?: boolean;
        class?: string;
        style?: string;
    }

    let { value, onchange, disabled = false, class: className = '', style = '' }: Props = $props();

    let editing = $state(false);
    let localValue = $state<string | undefined | null>();

    function handleKeyDown(e: KeyboardEvent) {
        if (disabled) return;
        if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            localValue = value;
            editing = true;
        }
    }

    function handleClick() {
        if (!disabled) {
            localValue = value;
            editing = true;
        }
    }

    function handleTextareaChange(e: Event & { currentTarget: EventTarget & HTMLTextAreaElement }) {
        localValue = e.currentTarget.value;
    }

    function handleBlur() {
        editing = false;
        if (localValue !== value) onchange(localValue ?? "");
    }
    
    function focusAction(node: HTMLTextAreaElement) {
        node.focus();
    }
</script>

{#if !editing}
    <button
        type="button"
        {disabled}
        onkeydown={handleKeyDown}
        onclick={handleClick}
        class={className}
        style={`min-height: 24px; cursor: ${disabled ? "not-allowed" : "text"}; padding: 0; color: ${value ? "var(--fg)" : "var(--fg-dim)"}; border-radius: 4px; background: none; border: none; font: inherit; text-align: left; width: 100%; ${style}`}
    >
        {value ?? "Add description..."}
    </button>
{:else}
    <textarea
        use:focusAction
        value={localValue ?? ""}
        oninput={handleTextareaChange}
        onblur={handleBlur}
        rows="5"
        class={className}
        style={`width: 100%; resize: vertical; padding: 4px; font-family: inherit; border: 1px solid var(--border); background: var(--bg-surface); color: var(--fg); border-radius: 4px; ${style}`}
        placeholder="Add a description..."
        spellcheck="false"
    ></textarea>
{/if}
