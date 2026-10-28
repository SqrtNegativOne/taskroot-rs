<script lang="ts">
    import { safeInvoke } from '$lib/safeInvoke.svelte';
    import { describeAppError } from '$lib/errors';
    import { notify } from '$lib/notifications.svelte';

    let isRefreshing = $state(false);

    async function refresh(): Promise<void> {
        if (isRefreshing) return;
        isRefreshing = true;
        const result = await safeInvoke('refresh_data');
        isRefreshing = false;
        if (result.isErr()) {
            notify(`Refresh failed: ${describeAppError(result.error)}`, 'error');
        }
    }
</script>

<button
    onclick={refresh}
    disabled={isRefreshing}
    style="padding: 0 4px; display: flex; background: transparent; border: none; cursor: pointer; color: inherit; align-items: center;"
    aria-label="Reload from CalDAV"
    title="Reload from CalDAV"
>
    <span class="material-symbols-outlined {isRefreshing ? 'spinning' : ''}" style="font-size: 18px;">
        refresh
    </span>
</button>
