<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { listen } from '@tauri-apps/api/event';
    import { onMount } from 'svelte';
    import { store } from '$lib/store.svelte';

    let isSyncing = $state(false);

    onMount(() => {
        let unlistenStart: (() => void) | undefined;
        let unlistenFinish: (() => void) | undefined;
        let unlistenError: (() => void) | undefined;

        listen('sync-started', () => {
            isSyncing = true;
        }).then(u => unlistenStart = u);

        listen('sync-finished', () => {
            isSyncing = false;
            store.refresh();
        }).then(u => unlistenFinish = u);

        listen('sync-error', (err) => {
            isSyncing = false;
            console.error("Background sync error:", err);
        }).then(u => unlistenError = u);

        return () => {
            if (unlistenStart) unlistenStart();
            if (unlistenFinish) unlistenFinish();
            if (unlistenError) unlistenError();
        };
    });

    async function handleSync() {
        if (isSyncing) return;
        isSyncing = true;
        try {
            await invoke('force_sync');
        } catch (e) {
            console.error("Force sync failed:", e);
        } finally {
            isSyncing = false;
        }
    }
</script>

<button class="stage" onclick={handleSync} disabled={isSyncing} style="padding: 0 4px; display: flex; background: transparent; border: none; cursor: pointer; color: inherit; align-items: center;" aria-label="Sync">
    <span class="material-symbols-outlined {isSyncing ? 'spinning' : ''}" style="font-size: 18px;">sync</span>
</button>
