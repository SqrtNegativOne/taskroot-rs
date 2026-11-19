<script lang="ts">
    import '../app.css';
    import { onMount } from 'svelte';
    import type { Snippet } from 'svelte';
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { page } from '$app/state';
    import { store, describeError } from '$lib/store.svelte';
    import { safeInvoke } from '$lib/safeInvoke.svelte';
    import { Routes } from '$lib/routes';
    import { LAUNCHER_NAVIGATE } from '$lib/ipcEvents';
    import TitleBar from '../components/TitleBar.svelte';
    import TaskListState from '../lib/TaskListState.svelte';
    import LauncherPalette from '$lib/launcher/LauncherPalette.svelte';
    import { Toaster } from 'svelte-sonner';

    import { listen } from '@tauri-apps/api/event';
    import type { RouteId } from '$app/types';
    import type { WindowLabel } from '$lib/bindings/WindowLabel.generated';

    function isRouteId(value: string): value is RouteId {
        return (Object.values(Routes) as string[]).includes(value);
    }

    let { children }: { children: Snippet } = $props();
    let isLauncher = $state(false);
    let isMinitracker = $state(false);
    let isSidebar = $state(false);
    let isEffect = $state(false);
    let isBooting = $state(true);

    onMount(() => {
        const appWindow = getCurrentWindow();
        const label = appWindow.label as WindowLabel;
        isLauncher = label === 'launcher';
        isMinitracker = label === 'minitracker';
        isSidebar = (label as string) === 'sidebar';
        isEffect = label === 'effect';

        let unlistenNav = () => { /* noop */ };

        const redirectForAccount = async () => {
            const configured = await safeInvoke<boolean>('is_account_configured');
            if (configured.isErr()) {
                store.error = `Could not check the CalDAV account: ${describeError(configured.error)}`;
                return;
            }

            const onConnect = page.url.pathname === Routes.CONNECT;
            if (!configured.value && !onConnect) {
                await goto(resolve(Routes.CONNECT));
                return;
            }
            if (configured.value && onConnect) {
                await goto(resolve(Routes.HOME));
            }
        };

        const initialize = async () => {
            const initResult = await store.init();
            if (initResult.isErr()) {
                store.error = `Error loading data from backend: ${describeError(initResult.error)}`;
            }

            if (!isLauncher && !isMinitracker && !isSidebar && !isEffect) {
                await redirectForAccount();
            }

            isBooting = false;

            if (!isLauncher && !isMinitracker && !isSidebar && !isEffect) {
                unlistenNav = await listen(LAUNCHER_NAVIGATE, (event) => {
                    const route = event.payload as string;
                    if (isRouteId(route)) {
                        void (async () => {
                            await goto(resolve(route));
                            await safeInvoke('window_restore_main');
                        })();
                    }
                });
            }
        };

        void initialize();

        return () => {
            unlistenNav();
        };
    });
</script>

{#if isLauncher}
    <LauncherPalette />
{:else if isSidebar}
    <div class="sidebar-shell">
        {@render children()}
    </div>
{:else if isEffect}
    <div class="effect-shell">
        {@render children()}
    </div>
{:else if isBooting}
    <div class="boot-screen">
        <div class="boot-spinner"></div>
    </div>
{:else}
    <div class="app">
        <TaskListState />
        {#if !isMinitracker}
            <TitleBar />
        {/if}
        {@render children()}
    </div>
{/if}

{#if !isEffect}
    <Toaster richColors position="bottom-right" />
{/if}

<style>
    .sidebar-shell {
        height: 100vh;
        width: 100vw;
        background: transparent;
        overflow: hidden;
    }

    .effect-shell {
        height: 100vh;
        width: 100vw;
        background: transparent;
        overflow: hidden;
    }

    .boot-screen {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100vh;
        width: 100vw;
        background: var(--bg);
    }

    .boot-spinner {
        width: 32px;
        height: 32px;
        border: 2px solid var(--border);
        border-top-color: var(--fg-muted);
        border-radius: 50%;
        animation: boot-spin 1s linear infinite;
    }

    @keyframes boot-spin {
        from {
            transform: rotate(0deg);
        }
        to {
            transform: rotate(360deg);
        }
    }
</style>
