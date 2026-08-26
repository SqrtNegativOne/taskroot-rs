<script lang="ts">
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import { page } from '$app/state';
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { Routes } from '$lib/routes';
    import SyncButton from './SyncButton.svelte';
    import StageIndicator from './shell/StageIndicator.svelte';
    import MoreScreensDropdown from './shell/MoreScreensDropdown.svelte';
    import WindowControls from './shell/WindowControls.svelte';
    import type { WindowLabel } from '$lib/bindings/WindowLabel.generated';

    const appWindow = getCurrentWindow();
    const label = appWindow.label as WindowLabel;
    const isMinitracker = label === 'minitracker';

    function navigate(stage: 'settings' | 'home'): void {
        void goto(resolve(stage === 'home' ? Routes.HOME : Routes.SETTINGS));
    }
</script>

<header class="topbar">
    <div class="drag-region"></div>
    <div class="topbar-left" style="display: flex; align-items: center; gap: 8px;">
        <button onclick={() => navigate('settings')} class="stage {page.url.pathname === Routes.SETTINGS ? 'is-current' : ''}" style="padding: 0 4px; display: flex; background: none; border: none; cursor: pointer; color: inherit;" aria-label="Settings">
            <span class="material-symbols-outlined" style="font-size: 18px;">settings</span>
        </button>
        <StageIndicator />
        <MoreScreensDropdown />
    </div>
    <div class="topbar-right"></div>

    <WindowControls {isMinitracker}>
        <SyncButton />
    </WindowControls>
</header>
