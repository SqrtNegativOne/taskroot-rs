<script lang="ts">
    import '../app.css';
    import { onMount } from 'svelte';
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import { emit } from '@tauri-apps/api/event';
    import { store } from '$lib/store.svelte';
    import { useAppIntegration } from '$lib/useAppIntegration.svelte';
    import { safeInvoke } from '$lib/safeInvoke.svelte';
    import { goto } from '$app/navigation';
    import TitleBar from '../components/TitleBar.svelte';

    let { children } = $props();
    let isLauncher = $state(false);
    let isMinitracker = $state(false);
    let isCheckingAuth = $state(true);

    onMount(async () => {
        const appWindow = getCurrentWindow();
        isLauncher = appWindow.label === 'launcher';
        isMinitracker = appWindow.label === 'minitracker';
        
        if (!isLauncher && !isMinitracker) {
            const authResult = await safeInvoke<boolean>('is_logged_in');
            
            if (authResult.isOk()) {
                const loggedIn = authResult.value;
                if (!loggedIn && window.location.pathname !== '/login') {
                    await goto('/login');
                } else if (loggedIn && window.location.pathname === '/login') {
                    await goto('/plan');
                }
            } else {
                console.error("Failed to check auth state:", authResult.error);
            }
            
            isCheckingAuth = false;
            store.init(); // Initialize store only after auth check to avoid fetching without token
        } else {
            isCheckingAuth = false;
            store.init();
        }
    });

    useAppIntegration();

    // Effect to sync data to launcher when store changes
    $effect(() => {
        if (!isLauncher && !isMinitracker && store.loaded) {
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
    {#if isCheckingAuth}
        <div class="h-screen w-screen bg-zinc-950 flex items-center justify-center">
            <div class="w-8 h-8 border-2 border-zinc-800 border-t-zinc-400 rounded-full animate-spin"></div>
        </div>
    {:else}
        <div class="app">
            <TitleBar />
            {@render children()}
        </div>
    {/if}
{/if}
