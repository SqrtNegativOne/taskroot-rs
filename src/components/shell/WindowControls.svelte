<script lang="ts">
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import type { Snippet } from 'svelte';

    interface Props {
        children?: Snippet;
    }

    let { children }: Props = $props();

    const appWindow = getCurrentWindow();
    const handleMinimize = () => void appWindow.minimize();
    const handleMaximize = () => void appWindow.toggleMaximize();
    const handleClose = () => void appWindow.close();
</script>

<div class="window-controls">
    {@render children?.()}
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
