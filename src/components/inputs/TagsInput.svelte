<script lang="ts">
    interface Props {
        tags: string[];
        onchange: (tags: string[]) => void;
        placeholder?: string;
        class?: string;
        style?: string;
    }

    let { tags = [], onchange, placeholder = 'Add tag...', class: className = '', style = '' }: Props = $props();
    let inputValue = $state('');

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

<div class={className} style={`display: flex; flex-wrap: wrap; gap: 4px; background: var(--bg-input, transparent); border: 1px solid var(--border, #ccc); border-radius: 4px; padding: 4px; min-height: 32px; box-sizing: border-box; width: 100%; ${style}`}>
    {#each tags as tag}
        <span style="background: var(--accent, #007bff); color: var(--bg, #fff); padding: 2px 8px; border-radius: 12px; font-size: 12px; display: flex; align-items: center; gap: 4px; user-select: none;">
            {tag}
            <button 
                type="button" 
                onclick={() => removeTag(tag)} 
                style="background: none; border: none; color: inherit; cursor: pointer; padding: 0; font-size: 14px; line-height: 1; display: flex; align-items: center; justify-content: center;"
                aria-label={`Remove tag ${tag}`}
            >
                &times;
            </button>
        </span>
    {/each}
    <input
        type="text"
        {placeholder}
        bind:value={inputValue}
        onkeydown={handleKeyDown}
        style="flex: 1; min-width: 80px; background: transparent; border: none; color: var(--fg, inherit); outline: none; padding: 2px 4px; font-size: 14px;"
    />
</div>
