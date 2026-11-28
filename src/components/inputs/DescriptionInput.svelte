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
    let copied = $state(false);
    let copyReset: ReturnType<typeof setTimeout> | undefined;

    $effect(() => () => clearTimeout(copyReset));

    const hasContent = $derived(!!value && value.trim() !== '');

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
        if (!editing) return;
        editing = false;
        if (localValue !== (value ?? '')) onchange(localValue);
    }

    async function handleCopy() {
        if (!value) return;
        try {
            await navigator.clipboard.writeText(value);
            copied = true;
            clearTimeout(copyReset);
            copyReset = setTimeout(() => {
                copied = false;
            }, 1500);
        } catch (error) {
            console.error('Failed to copy description:', error);
        }
    }

    function focusAction(node: HTMLTextAreaElement) {
        node.focus();
    }
</script>

<div class="description-input">
    {#if !editing}
        <button
            type="button"
            {disabled}
            onkeydown={handleKeyDown}
            onclick={startEditing}
            class={`description-display ${className}`.trim()}
            class:is-empty={!hasContent}
            aria-label={ariaLabel}
            style={style}
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
            class={`description-editor ${className}`.trim()}
            style={style}
            aria-label={ariaLabel}
            placeholder={placeholder}
            spellcheck="false"
        ></textarea>
    {/if}
    {#if hasContent && !editing}
        <button
            type="button"
            class="description-copy"
            class:is-copied={copied}
            aria-label={copied ? 'Copied' : 'Copy description'}
            title={copied ? 'Copied' : 'Copy description'}
            onclick={handleCopy}
        >
            <span class="material-symbols-outlined" aria-hidden="true">{copied ? 'check' : 'content_copy'}</span>
        </button>
    {/if}
</div>

<style>
    .description-input {
        position: relative;
        width: 100%;
    }

    .description-display {
        display: block;
        width: 100%;
        min-height: 24px;
        padding: 0;
        border: none;
        border-radius: 0;
        background: none;
        color: inherit;
        font: inherit;
        text-align: left;
        white-space: pre-wrap;
        overflow-wrap: anywhere;
        cursor: text;
    }

    .description-display.is-empty {
        color: var(--fg-dim);
    }

    .description-display:disabled {
        color: var(--fg);
        cursor: not-allowed;
        opacity: 0.55;
    }

    .description-editor {
        width: 100%;
        padding: 6px 8px;
        border: 1px solid var(--border);
        border-radius: 0;
        background: var(--bg-surface);
        color: var(--fg);
        font-family: inherit;
        font-size: inherit;
        resize: vertical;
    }

    .description-editor:focus {
        border-color: var(--accent);
        outline: none;
    }

    .description-copy {
        position: absolute;
        top: 0;
        right: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 2px;
        border: none;
        border-radius: 0;
        background: none;
        color: var(--fg-dim);
        cursor: pointer;
        opacity: 0;
        transition: opacity 0.12s ease, color 0.12s ease;
    }

    .description-input:hover .description-copy,
    .description-copy:focus-visible,
    .description-copy.is-copied {
        opacity: 1;
    }

    .description-copy:hover:not(:disabled) {
        color: var(--fg);
    }

    .description-copy.is-copied {
        color: var(--p0);
    }

    .description-copy .material-symbols-outlined {
        font-size: 16px;
    }
</style>
