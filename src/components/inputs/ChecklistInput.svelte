<script lang="ts">
    import type { ChecklistItem } from '../../lib/domain';

    interface Props {
        id?: string;
        checklist: ChecklistItem[];
        onchange: (checklist: ChecklistItem[]) => void;
    }

    let { id, checklist = [], onchange }: Props = $props();

    let newValue = $state('');

    function handleAdd(e: KeyboardEvent | FocusEvent) {
        if (e.type === 'keydown' && (e as KeyboardEvent).key !== 'Enter') return;
        
        const trimmed = newValue.trim();
        if (trimmed) {
            onchange([...checklist, { id: crypto.randomUUID(), title: trimmed, done: false }]);
            newValue = '';
        }
    }

    function toggleItem(index: number) {
        const next = [...checklist];
        next[index] = { ...next[index], done: !next[index].done };
        onchange(next);
    }

    function removeItem(index: number) {
        const next = [...checklist];
        next.splice(index, 1);
        onchange(next);
    }
</script>

<div class="checklist-input" {id}>
    {#if checklist.length > 0}
        <div class="checklist-list">
            {#each checklist as item, i (item.id)}
                <div class="checklist-item">
                    <input 
                        type="checkbox" 
                        checked={item.done}
                        onchange={() => toggleItem(i)}
                    />
                    <span class="checklist-title" class:done={item.done}>{item.title}</span>
                    <button class="checklist-remove" onclick={() => removeItem(i)} title="Remove Item">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6L6 18M6 6l12 12"/></svg>
                    </button>
                </div>
            {/each}
        </div>
    {/if}
    <input
        type="text"
        class="checklist-new-input"
        placeholder="Add item..."
        bind:value={newValue}
        onkeydown={handleAdd}
        onblur={handleAdd}
    />
</div>

<style>
    .checklist-input {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
    .checklist-list {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }
    .checklist-item {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 4px 8px;
        background: var(--bg-1);
        border-radius: 4px;
        border: 1px solid var(--bg-2);
    }
    .checklist-title {
        flex: 1;
        font-size: 0.9em;
    }
    .checklist-title.done {
        text-decoration: line-through;
        opacity: 0.6;
    }
    .checklist-remove {
        background: transparent;
        border: none;
        color: var(--fg-dim);
        cursor: pointer;
        padding: 4px;
        border-radius: 4px;
        display: flex;
        align-items: center;
        justify-content: center;
    }
    .checklist-remove:hover {
        background: var(--bg-2);
        color: var(--fg);
    }
    .checklist-new-input {
        padding: 6px 8px;
        border: 1px solid var(--bg-2);
        border-radius: 4px;
        background: var(--bg-surface);
        color: var(--fg);
        font-size: 0.9em;
    }
    .checklist-new-input:focus {
        outline: none;
        border-color: var(--p0);
    }
</style>
