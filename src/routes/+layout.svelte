<script lang="ts">
    import '../app.css';
    import { onMount } from 'svelte';
    import type { Snippet } from 'svelte';
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { store, describeError } from '$lib/store.svelte';
    import { safeInvoke } from '$lib/safeInvoke.svelte';
    import { Routes } from '$lib/routes';
    import TitleBar from '../components/TitleBar.svelte';

    import { listen, emit } from '@tauri-apps/api/event';
    import { LAUNCHER_DATA_UPDATE } from '$lib/events';

    let { children }: { children: Snippet } = $props();
    let isLauncher = $state(false);
    let isMinitracker = $state(false);
    let isSidebar = $state(false);
    let isCheckingAuth = $state(true);
    let launcherValue = $state('');

    async function redirectToInitialRoute(isLoggedIn: boolean): Promise<void> {
        if (!isLoggedIn && window.location.pathname !== Routes.LOGIN) {
            await goto(resolve(Routes.LOGIN));
        } else if (isLoggedIn && window.location.pathname === Routes.LOGIN) {
            await goto(resolve(Routes.HOME));
        }
    }

    onMount(() => {
        const appWindow = getCurrentWindow();
        isLauncher = appWindow.label === 'launcher';
        isMinitracker = appWindow.label === 'minitracker';
        isSidebar = appWindow.label === 'sidebar';

        let unlistenData = () => { /* noop */ };
        let unlistenNav = () => { /* noop */ };

        const initialize = async () => {
            if (!isLauncher && !isMinitracker && !isSidebar) {
                const authResult = await safeInvoke<boolean>('is_logged_in');
                if (authResult.isOk()) {
                    await redirectToInitialRoute(authResult.value);
                } else {
                    console.error('Failed to check auth state:', authResult.error);
                }
            }

            const initResult = await store.init();
            if (initResult.isErr()) {
                store.error = `Error loading data from backend: ${describeError(initResult.error)}`;
            }

            isCheckingAuth = false;

            if (isLauncher) {
                unlistenData = await listen(LAUNCHER_DATA_UPDATE, (event) => {
                    console.log('launcher data update', event.payload);
                });
            } else if (!isMinitracker && !isSidebar) {
                unlistenNav = await listen('launcher-navigate', (event) => {
                    const route = event.payload as string;
                    const validRoutes = Object.values(Routes) as string[];
                    if (validRoutes.includes(route)) {
                        void (async () => {
                            // @ts-expect-error resolving dynamic string route
                            await goto(resolve(route));
                            await safeInvoke('window_restore_main');
                        })();
                    }
                });
            }
        };

        void initialize();

        return () => {
            unlistenData();
            unlistenNav();
        };
    });

    async function handleLauncherKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') {
            await safeInvoke('hide_launcher');
        } else if (e.key === 'Enter') {
            const cmd = launcherValue.trim().toLowerCase();
            const routeMap: Record<string, string> = {
                'home': Routes.HOME,
                'plan': Routes.HOME,
                'do': Routes.DO,
                'settings': Routes.SETTINGS,
            };
            
            if (routeMap[cmd]) {
                await emit('launcher-navigate', routeMap[cmd]);
                await safeInvoke('hide_launcher');
                launcherValue = '';
            } else {
                console.log('Unknown command:', cmd);
            }
        }
    }
</script>

{#if isLauncher}
    <div class="launcher-shell">
        <div class="launcher-panel">
            <span class="launcher-prefix">Cmd</span>
            <input 
                type="text" 
                class="launcher-input" 
                placeholder="Type a command..." 
                bind:value={launcherValue}
                onkeydown={handleLauncherKeydown}
            />
        </div>
    </div>
{:else if isSidebar}
    <div class="sidebar-shell">
        {@render children()}
    </div>
{:else if isCheckingAuth}
    <div class="boot-screen">
        <div class="boot-spinner"></div>
    </div>
{:else}
    <div class="app">
        {#if !isMinitracker}
            <TitleBar />
        {/if}
        {@render children()}
    </div>
{/if}

<style>
    .sidebar-shell {
        height: 100vh;
        width: 100vw;
        background: transparent;
        overflow: hidden;
    }

    .launcher-shell {
        height: 100vh;
        width: 100vw;
        padding: 8px;
        background: transparent;
    }

    .launcher-panel {
        display: flex;
        align-items: center;
        width: 100%;
        height: 100%;
        padding: 16px;
        color: var(--fg);
        background: var(--bg-surface);
        border: 1px solid var(--border-strong);
        border-radius: 8px;
        box-shadow: var(--shadow-btn-hover);
    }

    .launcher-prefix {
        margin-right: 12px;
        color: var(--fg-muted);
    }

    .launcher-input {
        flex: 1;
        font: inherit;
        font-size: 1.15em;
        color: inherit;
        background: transparent;
        border: none;
        outline: none;
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
