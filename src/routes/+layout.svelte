<script lang="ts">
    import '../app.css';
    import { onMount } from 'svelte';
    import type { Snippet } from 'svelte';
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { page } from '$app/state';
    import { store, describeError } from '$lib/store.svelte';
    import { commands, events, safeInvoke } from '$lib/safeInvoke.svelte';
    import { Routes } from '$lib/routes';
    import { claimFirstOpenOfDay } from '$lib/dailyStartup';
    import AccountBoundary from '../components/accounts/AccountBoundary.svelte';
    import TitleBar from '../components/TitleBar.svelte';
    import TaskListState from '../lib/TaskListState.svelte';
    import Theme from '$lib/themes/Theme.svelte';
    import LauncherPalette from '$lib/launcher/LauncherPalette.svelte';
    import { Toaster } from 'svelte-sonner';

    import type { RouteId } from '$app/types';
    import type { WindowLabel } from '$lib/domain';

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

        const isMainWindow = !isLauncher && !isMinitracker && !isSidebar && !isEffect;

        let disposed = false;
        let unlistenNav: (() => void) | undefined;

        const redirectForAccount = async (): Promise<boolean> => {
            const configured = await safeInvoke(commands.isAccountConfigured());
            if (disposed) return false;
            if (configured.isErr()) {
                store.error = `Could not check the CalDAV account: ${describeError(configured.error)}`;
                return false;
            }

            const onConnect = page.url.pathname === Routes.CONNECT;
            if (!configured.value && !onConnect) {
                await goto(resolve(Routes.CONNECT));
                return false;
            }
            if (configured.value && onConnect) {
                await goto(resolve(Routes.HOME));
            }
            return configured.value;
        };

        const redirectToStartup = async () => {
            if (page.url.pathname !== Routes.HOME) return;
            if (await claimFirstOpenOfDay() && !disposed) {
                await goto(resolve(Routes.STARTUP));
            }
        };

        const attachLauncherNavigation = async () => {
            const stop = await events.launcherNavigate.listen((event) => {
                if (disposed || !isRouteId(event.payload)) return;
                const route = event.payload;
                void (async () => {
                    await goto(resolve(route));
                    await safeInvoke(commands.windowRestoreMain());
                })();
            });
            if (disposed) stop();
            else unlistenNav = stop;
        };

        const initialize = async () => {
            const initResult = await store.init();
            if (disposed) return;
            if (initResult.isErr()) {
                store.error = `Error loading data from backend: ${describeError(initResult.error)}`;
            }

            if (isMainWindow) {
                const configured = await redirectForAccount();
                if (configured && !disposed) await redirectToStartup();
                if (disposed) return;
            }

            isBooting = false;
            if (isMainWindow) await attachLauncherNavigation();
        };

        void initialize();

        return () => {
            disposed = true;
            unlistenNav?.();
        };
    });
</script>

<Theme scheme={store.settings?.colour_scheme} />

{#if isLauncher}
    <AccountBoundary><LauncherPalette /></AccountBoundary>
{:else if isSidebar}
    <div class="sidebar-shell">
        <AccountBoundary>{@render children()}</AccountBoundary>
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
        <AccountBoundary preserve={page.url.pathname === Routes.SETTINGS}>
            {@render children()}
        </AccountBoundary>
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
