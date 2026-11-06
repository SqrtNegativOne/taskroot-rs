<script lang="ts">
    interface Props {
        id?: string;
        tags: string[];
        onchange: (tags: string[]) => void;
        placeholder?: string;
        class?: string;
        style?: string;
        variant?: 'default' | 'bare';
        ariaLabel?: string;
    }

    let {
        id,
        tags,
        onchange,
        placeholder = 'Add tag...',
        class: className = '',
        style = '',
        variant = 'default',
        ariaLabel,
    }: Props = $props();

    let inputValue = $state('');

    const containerStyle = $derived(
        variant === 'bare'
            ? `display: flex; flex-wrap: wrap; gap: 4px; padding: 0; min-height: 24px; width: 100%; ${style}`
            : `display: flex; flex-wrap: wrap; gap: 4px; background: var(--bg-surface); border: 1px solid var(--border, #ccc); border-radius: 4px; padding: 4px; min-height: 32px; box-sizing: border-box; width: 100%; ${style}`,
    );

    function handleKeyDown(e: KeyboardEvent) {
        if (e.key === 'Enter' && inputValue.trim()) {
            e.preventDefault();
            const newTag = inputValue.trim();
            if (!tags.includes(newTag)) {
                onchange([...tags, newTag]);
            }
            inputValue = '';
        } else if (e.key === 'Backspace' && !inputValue && tags.length > 0) {
            onchange(tags.slice(0, -1));
        }
    }

    function removeTag(tagToRemove: string) {
        onchange(tags.filter(t => t !== tagToRemove));
    }
</script>

<div class={className} style={containerStyle}>
    {#each tags as tag (tag)}
        <span style="background: var(--accent-soft); border: 1px solid var(--accent-line); color: var(--fg); padding: 1px 8px; border-radius: 10px; font-size: 12px; display: flex; align-items: center; gap: 4px; user-select: none;">
            {tag}
            <button
                type="button"
                onclick={() => removeTag(tag)}
                style="background: none; border: none; color: inherit; cursor: pointer; padding: 0; font-size: 14px; line-height: 1; display: flex; align-items: center; justify-content: center; opacity: 0.6;"
                aria-label={`Remove tag ${tag}`}
            >
                &times;
            </button>
        </span>
    {/each}
    <input
        {id}
        type="text"
        {placeholder}
        aria-label={ariaLabel}
        bind:value={inputValue}
        onkeydown={handleKeyDown}
        style="flex: 1; min-width: 80px; background: transparent; border: none; color: var(--fg, inherit); outline: none; padding: 2px 0; font-size: inherit; font-family: inherit;"
    />
</div>
