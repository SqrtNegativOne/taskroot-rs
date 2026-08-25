<script lang="ts">
    import { listen } from '@tauri-apps/api/event';
    import { onMount } from 'svelte';
    import { store } from '$lib/store.svelte';
    import { safeInvoke } from '$lib/safeInvoke.svelte';
    import type { SyncState } from '$lib/domain';
    import { SYNC_ERROR, SYNC_FINISHED, SYNC_STARTED } from '$lib/events';
    import { useNow } from '$lib/useNow.svelte';

    let isSyncing = $state(false);
    let syncError = $state<string | null>(null);
    let nextSyncAt = $state<Date | null>(null);

    const now = useNow();

    let countdownText = $derived.by(() => {
        if (!nextSyncAt) return '';
        const diffMs = nextSyncAt.getTime() - now.ms;
        if (diffMs <= 0) return ''; // If it's passed or syncing
        
        const m = Math.floor(diffMs / 60000);
        const s = Math.floor((diffMs % 60000) / 1000);
        return `${m.toString()}m ${s.toString()}s`;
    });

    let tooltipText = $derived.by(() => {
        const parts: string[] = [];
        if (syncError) {
            parts.push(`Error: ${syncError}`);
        }
        if (countdownText && !isSyncing) {
            parts.push(`Next sync in ${countdownText}`);
        } else if (nextSyncAt) {
            const timeStr = nextSyncAt.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
            parts.push(`Next sync: ${timeStr}`);
        } else {
            parts.push('Next sync: Unknown');
        }
        return parts.join('\n');
    });

    onMount(() => {
        const updateState = async (): Promise<void> => {
            const res = await safeInvoke<SyncState>('get_sync_state');
            res.match(
                (state) => {
                    isSyncing = state.is_syncing;
                    syncError = state.error;
                    nextSyncAt = state.next_sync_at ? new Date(state.next_sync_at) : null;
                },
                (e) => console.error('Failed to get sync state:', e)
            );
        };

        const subscriptions = [
            listen(SYNC_STARTED, () => void updateState()),
            listen(SYNC_FINISHED, () => {
                void updateState();
                void store.refresh();
            }),
            listen(SYNC_ERROR, (err) => {
                void updateState();
                console.error('Background sync error:', err);
            })
        ];

        void updateState();

        return () => {
            void Promise.all(subscriptions).then((unlisteners) => {
                for (const unlisten of unlisteners) unlisten();
            });
        };
    });

    async function handleSync(): Promise<void> {
        if (isSyncing) return;
        isSyncing = true;
        const res = await safeInvoke('force_sync');
        if (res.isErr()) console.error('Force sync failed:', res.error);
        isSyncing = false;
    }
</script>

<button class="stage" onclick={handleSync} disabled={isSyncing} style="padding: 0 4px; display: flex; background: transparent; border: none; cursor: pointer; color: {syncError ? 'var(--tag-red)' : 'inherit'}; align-items: center; gap: 4px;" aria-label="Sync" title={tooltipText}>
    <span class="material-symbols-outlined {isSyncing ? 'spinning' : ''}" style="font-size: 18px;">sync</span>
</button>
