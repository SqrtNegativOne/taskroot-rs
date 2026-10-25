<script lang="ts">
    import { onMount } from 'svelte';
    import { safeInvoke } from '$lib/safeInvoke.svelte';
    import type { SyncState } from '$lib/domain';

    let syncState = $state<SyncState | null>(null);
    let lsContents = $state<Record<string, string | null>>({});

    async function fetchState() {
        const syncRes = await safeInvoke<SyncState>('get_sync_state');
        if (syncRes.isOk()) {
            syncState = syncRes.value;
        }

        const ls: Record<string, string | null> = {};
        for (let i = 0; i < localStorage.length; i++) {
            const key = localStorage.key(i);
            if (key) {
                ls[key] = localStorage.getItem(key);
            }
        }
        lsContents = ls;
    }

    onMount(() => {
        void fetchState();
        const interval = setInterval(fetchState, 2000);
        return () => clearInterval(interval);
    });

    async function handlePush() {
        await safeInvoke('force_sync');
        void fetchState();
    }

    async function handlePull() {
        await safeInvoke('force_sync');
        void fetchState();
    }

    async function wipeData() {
        if (confirm("Are you sure you want to wipe all local data?")) {
            await safeInvoke('wipe_local_data');
            window.location.reload();
        }
    }
</script>

<div class="dev-screen" style="padding: 20px; overflow-y: auto; height: 100%; box-sizing: border-box;">
    <h1>Developer Mode</h1>
    
    <section style="margin-bottom: 24px;">
        <h2>Sync Controls</h2>
        <div style="display: flex; gap: 8px;">
            <button onclick={handlePush} class="btn">Push Sync</button>
            <button onclick={handlePull} class="btn">Pull Sync</button>
        </div>
    </section>

    <section style="margin-bottom: 24px;">
        <h2>Danger Zone</h2>
        <div style="display: flex; gap: 8px;">
            <button onclick={wipeData} class="btn" style="background: var(--tag-red); color: white;">Wipe Local Data</button>
        </div>
    </section>

    <section style="margin-bottom: 24px;">
        <h2>SyncState Inspection</h2>
        <pre style="background: var(--bg-panel); padding: 12px; border-radius: 4px; font-size: 12px; overflow-x: auto;">
{JSON.stringify(syncState, undefined, 2)}
        </pre>
    </section>

    <section style="margin-bottom: 24px;">
        <h2>LocalStorage Inspection</h2>
        <pre style="background: var(--bg-panel); padding: 12px; border-radius: 4px; font-size: 12px; overflow-x: auto;">
{JSON.stringify(lsContents, undefined, 2)}
        </pre>
    </section>
</div>
