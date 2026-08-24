<script lang="ts">
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import { page } from '$app/state';
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { Routes } from '$lib/routes';
    import SyncButton from './SyncButton.svelte';

    const STAGE_ROUTES = {
        plan: Routes.HOME,
        do: Routes.DO,
        dev: Routes.DEV,
        docs: Routes.DOCS,
        graph: Routes.GRAPH,
        launcher: Routes.LAUNCHER,
        minitracker: Routes.MINITRACKER,
        recap: Routes.RECAP,
        settings: Routes.SETTINGS,
        stats: Routes.STATS,
        wrap: Routes.WRAP
    } as const;

    type Stage = keyof typeof STAGE_ROUTES;

    const stages = Object.keys(STAGE_ROUTES) as Stage[];

    function isStage(value: string): value is Stage {
        return value in STAGE_ROUTES;
    }

    const handleMinimize = () => getCurrentWindow().minimize();
    const handleMaximize = () => getCurrentWindow().toggleMaximize();
    const handleClose = () => getCurrentWindow().close();

    let current = $derived(page.url.pathname === Routes.HOME ? 'plan' : page.url.pathname.substring(1));

    function navigate(stage: Stage): void {
        void goto(resolve(STAGE_ROUTES[stage]));
    }

    function handleStageChange(event: Event & { currentTarget: EventTarget & HTMLSelectElement }): void {
        const stage = event.currentTarget.value;
        if (isStage(stage)) navigate(stage);
    }
</script>

<header class="topbar">
    <div class="drag-region"></div>
    <div class="topbar-left">
        <button onclick={() => navigate('settings')} class="stage {current === 'settings' ? 'is-current' : ''}" style="padding: 0 4px; display: flex; background: none; border: none; cursor: pointer; color: inherit;" aria-label="Settings">
            <span class="material-symbols-outlined" style="font-size: 18px;">settings</span>
        </button>
        <select class="stage-name" aria-label="Active stage" style="background: transparent; border: none; color: var(--fg); font: inherit; outline: none; cursor: pointer; padding: 0;" onchange={handleStageChange}>
            {#each stages as p (p)}
                <option value={p} selected={current === p} style="background: var(--bg-surface); color: var(--fg);">{p}</option>
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
