<script lang="ts">
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import { page } from '$app/stores';
    import { goto } from '$app/navigation';
    import { Routes } from '$lib/routes';
    import SyncButton from './SyncButton.svelte';

    const handleMinimize = () => getCurrentWindow().minimize();
    const handleMaximize = () => getCurrentWindow().toggleMaximize();
    const handleClose = () => getCurrentWindow().close();

    let current = $derived($page.url.pathname === Routes.HOME ? 'plan' : $page.url.pathname.substring(1));

    function navigate(stage: string) {
        if (stage === 'plan') {
            void goto(Routes.HOME);
        } else {
            const key = stage.toUpperCase() as keyof typeof Routes;
            if (Routes[key]) {
                void goto(Routes[key]);
            } else {
                void goto(`/${stage}`);
            }
        }
    }
</script>

<header class="topbar">
    <div class="drag-region"></div>
    <div class="topbar-left">
        <button onclick={() => navigate('settings')} class="stage {current === 'settings' ? 'is-current' : ''}" style="padding: 0 4px; display: flex; background: none; border: none; cursor: pointer; color: inherit;" aria-label="Settings">
            <span class="material-symbols-outlined" style="font-size: 18px;">settings</span>
        </button>
        <select class="stage-name" style="background: none; border: none; color: inherit; font: inherit; outline: none; cursor: pointer; padding: 0;" onchange={(e) => { navigate(e.currentTarget.value); }}>
            {#each ['plan', 'do', 'dev', 'docs', 'graph', 'launcher', 'minitracker', 'recap', 'settings', 'stats', 'wrap'] as p}
                <option value={p} selected={current === p}>{p}</option>
            {/each}
        </select>
    </div>
    <div class="topbar-right"></div>
    
    <div class="window-controls">
        <SyncButton />
        <button class="win-btn minimize" onclick={handleMinimize} title="Minimize" aria-label="Minimize">
            <svg width="10" height="10" viewBox="0 0 10 10">
                <path d="M 1,5 h 8" stroke="currentColor" stroke-width="1" />
            </svg>
        </button>
        <button class="win-btn maximize" onclick={handleMaximize} title="Maximize" aria-label="Maximize">
            <svg width="10" height="10" viewBox="0 0 10 10">
                <rect x="1.5" y="1.5" width="7" height="7" fill="none" stroke="currentColor" stroke-width="1" />
            </svg>
        </button>
        <button class="win-btn close" onclick={handleClose} title="Close" aria-label="Close">
            <svg width="10" height="10" viewBox="0 0 10 10">
                <path d="M 1.5,1.5 l 7,7 M 8.5,1.5 l -7,7" stroke="currentColor" stroke-width="1" />
            </svg>
        </button>
    </div>
</header>
