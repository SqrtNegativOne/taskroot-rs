<script lang="ts">
    import { createSidebarState, saveTabTop } from './state.svelte';

    const state = createSidebarState();
</script>

<div data-testid="notes">{state.notes}</div>
<div data-testid="tab-top">{state.tabTop}</div>
<div data-testid="show-notes">{String(state.showNotes)}</div>
<div data-testid="hydrated">{String(state.hydrated)}</div>
<button onclick={() => { state.notes = `${state.notes}!`; }}>edit-notes</button>
<button onclick={() => saveTabTop(42)}>save-tab</button>
