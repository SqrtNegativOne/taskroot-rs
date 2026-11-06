<script lang="ts">
    interface Props {
        value: string | undefined | null;
        onchange: (val: string) => void;
        disabled?: boolean;
        class?: string;
        style?: string;
        placeholder?: string;
        ariaLabel?: string;
    }

    let {
        value,
        onchange,
        disabled = false,
        class: className = '',
        style = '',
        placeholder = 'Add description...',
        ariaLabel = 'Description',
    }: Props = $props();

    let editing = $state(false);
    let localValue = $state('');

    function startEditing() {
        if (disabled) return;
        localValue = value ?? '';
        editing = true;
    }

    function handleKeyDown(e: KeyboardEvent) {
        if (disabled) return;
        if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            startEditing();
        }
    }

    function handleTextareaChange(e: Event & { currentTarget: EventTarget & HTMLTextAreaElement }) {
        localValue = e.currentTarget.value;
    }

    function handleBlur() {
        editing = false;
        if (localValue !== (value ?? '')) onchange(localValue);
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
        onclick={startEditing}
        class={className}
        aria-label={ariaLabel}
        style={`min-height: 24px; cursor: ${disabled ? 'not-allowed' : 'text'}; padding: 0; color: ${value ? 'var(--fg)' : 'var(--fg-dim)'}; border-radius: 4px; background: none; border: none; font: inherit; text-align: left; width: 100%; white-space: pre-wrap; ${style}`}
    >
        {value || placeholder}
    </button>
{:else}
    <textarea
        use:focusAction
        value={localValue}
        oninput={handleTextareaChange}
        onblur={handleBlur}
        rows="5"
        class={className}
        aria-label={ariaLabel}
        style={`width: 100%; resize: vertical; padding: 6px 8px; font-family: inherit; font-size: inherit; border: 1px solid var(--border); background: var(--bg-surface); color: var(--fg); border-radius: 6px; ${style}`}
        placeholder={placeholder}
        spellcheck="false"
    ></textarea>
{/if}
