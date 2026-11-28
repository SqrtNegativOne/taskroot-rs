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
        suggestions?: string[];
        disabled?: boolean;
    }

    const uid = $props.id();
    const listboxId = `${uid}-tags-listbox`;

    let {
        id, tags, onchange, placeholder = 'Add tag...', class: className = '',
        style = '', variant = 'default', ariaLabel, suggestions = [], disabled = false,
    }: Props = $props();

    let inputValue = $state('');
    let isFocused = $state(false);
    let dismissed = $state(false);
    let highlightedIndex = $state(-1);

    const filteredSuggestions = $derived.by(() => {
        const selected = new Set(tags);
        const unique: string[] = [];
        const seen = new Set<string>();
        for (const candidate of suggestions) {
            const trimmed = candidate.trim();
            if (!trimmed || selected.has(trimmed) || seen.has(trimmed)) continue;
            seen.add(trimmed);
            unique.push(trimmed);
        }
        const query = inputValue.trim().toLowerCase();
        if (!query) return unique;
        const prefixMatches: string[] = [];
        const substringMatches: string[] = [];
        for (const candidate of unique) {
            const lower = candidate.toLowerCase();
            if (lower.startsWith(query)) prefixMatches.push(candidate);
            else if (lower.includes(query)) substringMatches.push(candidate);
        }
        return [...prefixMatches, ...substringMatches];
    });

    const dropdownOpen = $derived(isFocused && !disabled && !dismissed && filteredSuggestions.length > 0);
    const activeIndex = $derived(
        highlightedIndex >= 0 && highlightedIndex < filteredSuggestions.length ? highlightedIndex : -1,
    );
    const activeDescendant = $derived(
        dropdownOpen && activeIndex >= 0 ? `${listboxId}-opt-${activeIndex}` : undefined,
    );
    const suggestionsAriaLabel = $derived(ariaLabel ? `${ariaLabel} suggestions` : 'Tag suggestions');

    function addTag(raw: string): void {
        if (disabled) return;
        const trimmed = raw.trim();
        if (!trimmed) return;
        if (!tags.includes(trimmed)) onchange([...tags, trimmed]);
        inputValue = '';
        highlightedIndex = -1;
        dismissed = false;
    }

    function removeTag(tagToRemove: string): void {
        if (!disabled) onchange(tags.filter((tag) => tag !== tagToRemove));
    }

    function moveHighlight(delta: 1 | -1): void {
        const count = filteredSuggestions.length;
        if (count === 0) return;
        dismissed = false;
        highlightedIndex = activeIndex < 0 ? (delta === 1 ? 0 : count - 1) : (activeIndex + delta + count) % count;
    }

    function commitOnEnter(e: KeyboardEvent): void {
        const selected = activeIndex >= 0 ? filteredSuggestions[activeIndex] : undefined;
        if (dropdownOpen && selected) {
            e.preventDefault();
            addTag(selected);
            return;
        }
        if (inputValue.trim()) {
            e.preventDefault();
            addTag(inputValue);
        }
    }

    function dismissDropdown(e: KeyboardEvent): void {
        if (!dropdownOpen) return;
        e.preventDefault();
        e.stopPropagation();
        dismissed = true;
        highlightedIndex = -1;
    }

    function handleKeyDown(e: KeyboardEvent): void {
        if (disabled) return;
        switch (e.key) {
            case 'ArrowDown': e.preventDefault(); moveHighlight(1); break;
            case 'ArrowUp': e.preventDefault(); moveHighlight(-1); break;
            case 'Escape': dismissDropdown(e); break;
            case 'Enter': commitOnEnter(e); break;
            case 'Backspace': if (!inputValue && tags.length > 0) onchange(tags.slice(0, -1)); break;
            default: break;
        }
    }
</script>

<div class={`tags-input-wrapper ${className}`.trim()} {style}>
    <div class="tags-input-box" class:is-bare={variant === 'bare'} class:is-disabled={disabled}>
        {#each tags as tag (tag)}
            <span class="tags-chip">
                {tag}
                <button
                    type="button"
                    class="tags-chip-remove"
                    {disabled}
                    onclick={() => removeTag(tag)}
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
            {disabled}
            role="combobox"
            aria-label={ariaLabel}
            aria-autocomplete="list"
            aria-expanded={dropdownOpen}
            aria-controls={dropdownOpen ? listboxId : undefined}
            aria-activedescendant={activeDescendant}
            bind:value={inputValue}
            oninput={() => { dismissed = false; highlightedIndex = -1; }}
            onfocus={() => { isFocused = true; dismissed = false; }}
            onblur={() => { isFocused = false; highlightedIndex = -1; }}
            onkeydown={handleKeyDown}
            class="tags-input-field"
        />
    </div>

    {#if dropdownOpen}
        <div id={listboxId} role="listbox" aria-label={suggestionsAriaLabel} class="tags-dropdown">
            {#each filteredSuggestions as suggestion, index (suggestion)}
                <button
                    type="button"
                    id={`${listboxId}-opt-${index}`}
                    role="option"
                    aria-selected={index === activeIndex}
                    class="tags-option"
                    class:is-active={index === activeIndex}
                    onpointerdown={(e) => e.preventDefault()}
                    onmousedown={(e) => e.preventDefault()}
                    onclick={() => addTag(suggestion)}
                >
                    {suggestion}
                </button>
            {/each}
        </div>
    {/if}
</div>

<style>
    .tags-input-wrapper { position: relative; width: 100%; }
    .tags-input-box {
        display: flex; flex-wrap: wrap; gap: 4px; width: 100%; min-height: 32px;
        padding: 4px; box-sizing: border-box; border-radius: 4px;
        background: var(--bg-surface); border: 1px solid var(--border);
    }
    .tags-input-box.is-bare { background: transparent; border: none; border-radius: 0; padding: 0; min-height: 24px; }
    .tags-input-box.is-disabled { opacity: 0.65; }
    .tags-chip {
        display: flex; align-items: center; gap: 4px; padding: 1px 8px; border-radius: 10px;
        background: var(--accent-soft); border: 1px solid var(--accent-line); color: var(--fg);
        font-size: 12px; user-select: none;
    }
    .tags-chip-remove {
        display: flex; align-items: center; justify-content: center; padding: 0; border: none;
        background: none; color: inherit; font-size: 14px; line-height: 1; cursor: pointer; opacity: 0.6;
    }
    .tags-chip-remove:disabled { cursor: not-allowed; opacity: 0.35; }
    .tags-input-field {
        flex: 1; min-width: 80px; padding: 2px 0; border: none; outline: none;
        background: transparent; color: var(--fg, inherit); font-size: inherit; font-family: inherit;
    }
    .tags-input-field:disabled { cursor: not-allowed; }
    .tags-dropdown {
        position: absolute; top: calc(100% + 4px); left: 0; right: 0; z-index: 1001;
        display: flex; flex-direction: column; gap: 2px; max-height: 180px; overflow-y: auto; padding: 4px;
        background: var(--bg-surface); border: 1px solid var(--border); border-radius: 6px;
        box-shadow: 0 6px 16px var(--shadow-medium);
    }
    .tags-option {
        display: flex; align-items: center; width: 100%; padding: 5px 8px; border: 1px solid transparent;
        border-radius: 4px; background: transparent; color: var(--fg); font: inherit; font-size: 0.9em;
        text-align: left; cursor: pointer;
    }
    .tags-option:hover { background: var(--bg-surface-hover); }
    .tags-option.is-active { background: var(--accent-soft); border-color: var(--accent-line); }
</style>
