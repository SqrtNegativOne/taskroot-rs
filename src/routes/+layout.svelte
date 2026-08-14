<script lang="ts">
    import '../app.css';
    import { onMount } from 'svelte';
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import { emit } from '@tauri-apps/api/event';
    import { store } from '$lib/store.svelte';
    import { useAppIntegration } from '$lib/useAppIntegration.svelte';

    let { children } = $props();
    let isLauncher = $state(false);

    onMount(() => {
        store.init();
        const appWindow = getCurrentWindow();
        isLauncher = appWindow.label === 'launcher';
    });

    useAppIntegration();

    // Effect to sync data to launcher when store changes
    $effect(() => {
        if (!isLauncher && store.loaded) {
            emit('launcher-data-update', {
                tasks: $state.snapshot(store.tasks),
                events: $state.snapshot(store.events)
            });
        }
    });
</script>

{#if isLauncher}
    <div class="h-screen w-screen bg-transparent p-2">
        <div class="w-full h-full bg-zinc-900 rounded-lg shadow-lg border border-zinc-700 p-4 text-white flex items-center">
            <!-- Temporary Launcher UI Placeholder -->
            <span class="text-zinc-400 mr-3">Cmd</span>
            <input type="text" class="bg-transparent outline-none flex-1 text-lg" placeholder="Type a command..." />
        </div>
    </div>
{:else}
    {@render children()}
{/if}
